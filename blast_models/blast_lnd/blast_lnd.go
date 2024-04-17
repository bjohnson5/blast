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

	pb "blast_lnd/blast_proto" // Import your generated proto file
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
		log.Println("Received shutdown signal")
		server.GracefulStop()
	}()

	wg.Wait()
}

func (blnd *BlastLnd) start_nodes(num_nodes int) {
	shutdownInterceptor, err := signal.Intercept()
	if err != nil {
		fmt.Println("Could not set up shutdown interceptor.")
		os.Exit(1)
	}

	dir, err := filepath.Abs(filepath.Dir(os.Args[0]))
	if err != nil {
		fmt.Println("Could not get executable directory.")
		os.Exit(1)
	}

	blast_data_dir := dir + "/blast_data"
	node_list := SimJsonFile{Nodes: []SimLnNode{}}

	for i := 0; i < num_nodes; i++ {
		node_id := pad_with_zeros(i, 4)
		fmt.Println("Creating node: " + node_id)

		lnddir := blast_data_dir + "/lnd" + node_id + "/"
		write_config(node_id, blast_data_dir, lnddir)
		lnd.DefaultLndDir = lnddir
		lnd.DefaultConfigFile = lnddir + "lnd.conf"
		loadedConfig, err := lnd.LoadConfig(shutdownInterceptor)
		if err != nil {
			if e, ok := err.(*flags.Error); !ok || e.Type != flags.ErrHelp {
				fmt.Println("Error loading config.")
				os.Exit(1)
			}
			os.Exit(0)
		}
		implCfg := loadedConfig.ImplementationConfig(shutdownInterceptor)

		n := SimLnNode{Id: "blast-" + node_id, Address: "localhost:4" + node_id, Macaroon: "", Cert: blast_data_dir + "/lnd" + node_id + "/tls.cert"}
		node_list.Nodes = append(node_list.Nodes, n)

		blnd.wg.Add(1)
		go start_lnd(loadedConfig, implCfg, shutdownInterceptor, blnd.wg)

		if i%10 == 0 {
			fmt.Println("Sleeping for 10 seconds...")
			time.Sleep(10 * time.Second)
		}
	}

	for _, n := range node_list.Nodes {
		var tlsCreds credentials.TransportCredentials
		tlsCreds, err = credentials.NewClientTLSFromFile(n.Cert, "")
		if err != nil {
			fmt.Println("error reading TLS cert: %w", err)
		}

		opts := []grpc.DialOption{
			grpc.WithBlock(),
			grpc.WithTransportCredentials(tlsCreds),
		}
		client, err := grpc.Dial(n.Address, opts...)
		if err != nil {
			fmt.Println("Error connecting to node: " + n.Id)
		}

		blnd.clients[n.Id] = client
	}

	blnd.create_sim_ln_data(node_list, blast_data_dir+"/sim.json")
}

func (blnd *BlastLnd) create_sim_ln_data(obj interface{}, filename string) error {
	// Marshal object to JSON
	jsonData, err := json.MarshalIndent(obj, "", "    ")
	if err != nil {
		return err
	}

	// Write JSON data to file
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

func write_config(num string, currentdir string, dir string) {
	// Define the target file path and content
	filePath := dir + "lnd.conf"
	conf := fmt.Sprintf(string(configfile), num, currentdir)

	// Ensure the directory structure exists
	err := ensure_directory_exists(filePath)
	if err != nil {
		fmt.Println("Error creating directories:", err)
		return
	}

	// Open the file for writing
	file, err := os.Create(filePath)
	if err != nil {
		fmt.Println("Error creating file:", err)
		return
	}
	defer file.Close()

	// Write the content to the file
	_, err = file.WriteString(conf)
	if err != nil {
		fmt.Println("Error writing to file:", err)
		return
	}

	fmt.Println("File created successfully at:", filePath)
}

func ensure_directory_exists(filePath string) error {
	dir := filepath.Dir(filePath)
	return os.MkdirAll(dir, os.ModePerm)
}

func start_grpc_server(wg *sync.WaitGroup, blnd *BlastLnd) *grpc.Server {
	// Create a new gRPC server
	server := grpc.NewServer()

	// Register your service with the server
	pb.RegisterBlastRpcServer(server, &BlastRpcServer{blast_lnd: blnd})

	// Define the address and start the server
	address := "localhost:50051"
	listener, err := net.Listen("tcp", address)
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}

	wg.Add(1)
	go func() {
		defer wg.Done()
		log.Printf("Server started at %s", address)
		if err := server.Serve(listener); err != nil {
			log.Fatalf("Failed to serve: %v", err)
		}
	}()

	return server
}

func start_lnd(cfg *lnd.Config, implCfg *lnd.ImplementationCfg, interceptor signal.Interceptor, wg *sync.WaitGroup) {
	defer wg.Done()

	// Call the "real" main in a nested manner so the defers will properly
	// be executed in the case of a graceful shutdown.
	if err := lnd.Main(
		cfg, lnd.ListenerCfg{}, implCfg, interceptor,
	); err != nil {
		_, _ = fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
