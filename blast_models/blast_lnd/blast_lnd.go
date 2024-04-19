package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	s "os/signal"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/jessevdk/go-flags"
	"github.com/lightningnetwork/lnd"
	"github.com/lightningnetwork/lnd/signal"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"

	pb "blast_lnd/blast_proto"
)

const configfile string = `listen=3%[1]s
rpclisten=localhost:4%[1]s
restlisten=localhost:5%[1]s
datadir=%[2]s/lnd%[1]s/data
logdir=%[2]s/lnd%[1]s/log
tlscertpath=%[2]s/lnd%[1]s/tls.cert
tlskeypath=%[2]s/lnd%[1]s/tls.key
letsencryptdir=%[2]s/lnd%[1]s/letsencrypt
watchtower.towerdir=%[2]s/lnd%[1]s/data/watchtower
noseedbackup=true
no-macaroons=true
accept-keysend=1

[bitcoin]
bitcoin.active=1
bitcoin.node=neutrino
bitcoin.regtest=1

[neutrino]
neutrino.connect=localhost:18444`

type SimLnNode struct {
	Id       string `json:"id"`
	Address  string `json:"address"`
	Macaroon string `json:"macaroon"`
	Cert     string `json:"cert"`
}

type SimJsonFile struct {
	Nodes []SimLnNode `json:"nodes"`
}

type BlastLnd struct {
	clients    map[string]*grpc.ClientConn
	simln_data []byte
	wg         *sync.WaitGroup
}

func main() {
	go func() {
		http.ListenAndServe("localhost:6060", nil)
	}()

	var wg sync.WaitGroup

	// Handle OS signals for graceful shutdown
	sigCh := make(chan os.Signal, 1)
	signalsToCatch := []os.Signal{
		os.Interrupt,
		os.Kill,
		syscall.SIGTERM,
		syscall.SIGQUIT,
		syscall.SIGKILL,
		syscall.SIGINT,
	}
	s.Notify(sigCh, signalsToCatch...)

	blast_lnd := BlastLnd{clients: make(map[string]*grpc.ClientConn), wg: &wg}
	server := start_grpc_server(&wg, &blast_lnd)

	wg.Add(1)
	go func() {
		defer wg.Done()
		// Wait for OS signal
		<-sigCh
		blast_lnd_log("Received shutdown signal")
		server.GracefulStop()
	}()

	blast_lnd_log("Model started")

	wg.Wait()
}

func (blnd *BlastLnd) start_nodes(num_nodes int) error {
	shutdownInterceptor, err := signal.Intercept()
	if err != nil {
		blast_lnd_log("Could not set up shutdown interceptor.")
		return err
	}

	dir, err := filepath.Abs(filepath.Dir(os.Args[0]))
	if err != nil {
		blast_lnd_log("Could not get executable directory.")
		return err
	}

	blast_data_dir := dir + "/blast_data"
	node_list := SimJsonFile{Nodes: []SimLnNode{}}

	for i := 0; i < num_nodes; i++ {
		node_id := pad_with_zeros(i, 4)
		lnddir := blast_data_dir + "/lnd" + node_id + "/"
		err := write_config(node_id, blast_data_dir, lnddir)
		if err != nil {
			blast_lnd_log("Error writing lnd config to file: " + err.Error())
			return err
		}
		lnd.DefaultLndDir = lnddir
		lnd.DefaultConfigFile = lnddir + "lnd.conf"
		loadedConfig, err := lnd.LoadConfig(shutdownInterceptor)
		if err != nil {
			if e, ok := err.(*flags.Error); !ok || e.Type != flags.ErrHelp {
				blast_lnd_log("Error loading config.")
			}
			return err
		}
		implCfg := loadedConfig.ImplementationConfig(shutdownInterceptor)

		n := SimLnNode{Id: "blast-" + node_id, Address: "localhost:4" + node_id, Macaroon: "/home/admin.macaroon", Cert: blast_data_dir + "/lnd" + node_id + "/tls.cert"}
		node_list.Nodes = append(node_list.Nodes, n)

		blnd.wg.Add(1)
		go start_lnd(loadedConfig, implCfg, shutdownInterceptor, blnd.wg)

		if i%10 == 0 {
			time.Sleep(10 * time.Second)
		}
	}

	for i, n := range node_list.Nodes {
		var tlsCreds credentials.TransportCredentials
		tlsCreds, err = credentials.NewClientTLSFromFile(n.Cert, "")
		if err != nil {
			blast_lnd_log("Error reading TLS cert" + err.Error())
			continue
		}

		opts := []grpc.DialOption{
			grpc.WithBlock(),
			grpc.WithTransportCredentials(tlsCreds),
		}
		client, err := grpc.Dial(n.Address, opts...)
		if err != nil {
			blast_lnd_log("Error connecting to node: " + n.Id)
			continue
		}

		blnd.clients[n.Id] = client
		node_id := pad_with_zeros(i, 4)
		node_list.Nodes[i].Address = "https://" + "127.0.0.1:4" + node_id
	}

	err = blnd.create_sim_ln_data(node_list, blast_data_dir+"/sim.json")
	if err != nil {
		blast_lnd_log("Error creating simln data" + err.Error())
		return err
	}

	return nil
}

func (blnd *BlastLnd) create_sim_ln_data(obj interface{}, filename string) error {
	jsonData, err := json.MarshalIndent(obj, "", "    ")
	if err != nil {
		return err
	}

	file, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer file.Close()

	_, err = file.Write(jsonData)
	if err != nil {
		return err
	}

	blnd.simln_data = jsonData
	return nil
}

func blast_lnd_log(message string) {
	log.Println("[BLAST MODEL: blast_lnd] " + message)
}

func pad_with_zeros(num int, width int) string {
	numStr := strconv.Itoa(num)
	numLen := len(numStr)
	if numLen >= width {
		return numStr
	}
	padding := width - numLen
	paddedStr := fmt.Sprintf("%s%s", strings.Repeat("0", padding), numStr)
	return paddedStr
}

func write_config(num string, currentdir string, dir string) error {
	filePath := dir + "lnd.conf"
	conf := fmt.Sprintf(string(configfile), num, currentdir)

	err := ensure_directory_exists(filePath)
	if err != nil {
		return err
	}

	file, err := os.Create(filePath)
	if err != nil {
		return err
	}
	defer file.Close()

	_, err = file.WriteString(conf)
	if err != nil {
		return err
	}

	return nil
}

func ensure_directory_exists(filePath string) error {
	dir := filepath.Dir(filePath)
	return os.MkdirAll(dir, os.ModePerm)
}

func start_grpc_server(wg *sync.WaitGroup, blnd *BlastLnd) *grpc.Server {
	server := grpc.NewServer()
	pb.RegisterBlastRpcServer(server, &BlastRpcServer{blast_lnd: blnd})

	address := "localhost:50051"
	listener, err := net.Listen("tcp", address)
	if err != nil {
		blast_lnd_log("Failed to listen: " + err.Error())
		return nil
	}

	wg.Add(1)
	go func() {
		defer wg.Done()
		blast_lnd_log("Server started at " + address)
		if err := server.Serve(listener); err != nil {
			blast_lnd_log("Failed to serve: " + err.Error())
		}
	}()

	return server
}

func start_lnd(cfg *lnd.Config, implCfg *lnd.ImplementationCfg, interceptor signal.Interceptor, wg *sync.WaitGroup) {
	defer wg.Done()

	if err := lnd.Main(cfg, lnd.ListenerCfg{}, implCfg, interceptor); err != nil {
		blast_lnd_log("Could not start lnd: " + err.Error())
	}
}
