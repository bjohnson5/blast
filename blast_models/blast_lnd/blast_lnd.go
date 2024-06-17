package main

import (
	"archive/tar"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"

	"golang.org/x/exp/slices"

	"github.com/jessevdk/go-flags"
	"github.com/lightningnetwork/lnd"
	"github.com/lightningnetwork/lnd/lnrpc"
	"github.com/lightningnetwork/lnd/signal"
	"github.com/phayes/freeport"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"

	pb "blast_lnd/blast_proto"
)

// TODO: fix hard coded ports, paths, ids, strings, etc...
// TODO: fix port numbers so that you can run blast_lnd without root permissions

const configfile string = `listen=%[3]s
rpclisten=localhost:%[4]s
datadir=%[2]s/lnd%[1]s/data
logdir=%[2]s/lnd%[1]s/log
tlscertpath=%[2]s/lnd%[1]s/tls.cert
tlskeypath=%[2]s/lnd%[1]s/tls.key
letsencryptdir=%[2]s/lnd%[1]s/letsencrypt
watchtower.towerdir=%[2]s/lnd%[1]s/data/watchtower
noseedbackup=true
no-macaroons=true
accept-keysend=1
debuglevel=error
trickledelay=1000
norest=true
alias=%[5]s

[rpcmiddleware]
rpcmiddleware.enable=false

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
	clients          map[string]lnrpc.LightningClient
	listen_addresses map[string]string
	rpc_addresses    map[string]string
	simln_data       []byte
	shutdown_ch      chan struct{}
	home_dir         string
	data_dir         string
	wg               *sync.WaitGroup
}

type LoadedNode struct {
	alias               string
	loadedConfig        *lnd.Config
	implCfg             *lnd.ImplementationCfg
	shutdownInterceptor signal.Interceptor
	listen_port         string
	rpc_port            string
}

var saved_ports map[string]*net.Listener

func main() {
	go func() {
		http.ListenAndServe("localhost:6060", nil)
	}()

	var wg sync.WaitGroup
	shutdown_channel := make(chan struct{})

	dir, err := filepath.Abs(filepath.Dir(os.Args[0]))
	if err != nil {
		blast_lnd_log("Could not get executable directory.")
		return
	}

	blast_data_dir := dir + "/blast_data"

	blast_lnd := BlastLnd{clients: make(map[string]lnrpc.LightningClient), listen_addresses: make(map[string]string), rpc_addresses: make(map[string]string), shutdown_ch: shutdown_channel, home_dir: dir, data_dir: blast_data_dir, wg: &wg}
	server := start_grpc_server(&wg, &blast_lnd)

	wg.Add(1)
	go func() {
		defer wg.Done()
		// Wait for shutdown signal
		<-shutdown_channel
		blast_lnd_log("Received shutdown signal")
		server.GracefulStop()
		os.RemoveAll(blast_lnd.data_dir)
	}()

	blast_lnd_log("Model started")

	wg.Wait()
}

func (blnd *BlastLnd) start_nodes(num_nodes int) error {
	blast_lnd_log("Starting nodes")

	shutdownInterceptor, err := signal.Intercept()
	if err != nil {
		blast_lnd_log("Could not set up shutdown interceptor.")
		return err
	}

	node_list := SimJsonFile{Nodes: []SimLnNode{}}
	used_ports := make([]int, 2*num_nodes)
	for i := 0; i < num_nodes; i++ {
		var listen_port string
		var rpc_port string
		node_id := pad_with_zeros(i, 4)
		lnddir := blnd.data_dir + "/lnd" + node_id + "/"
		used_ports, listen_port, rpc_port = get_ports(used_ports)
		alias := "blast-" + node_id
		err := write_config(node_id, blnd.data_dir, lnddir, listen_port, rpc_port, alias)
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

		n := SimLnNode{Id: alias, Address: "localhost:" + rpc_port, Macaroon: "/home/admin.macaroon", Cert: blnd.data_dir + "/lnd" + node_id + "/tls.cert"}
		node_list.Nodes = append(node_list.Nodes, n)
		blnd.listen_addresses[alias] = "localhost:" + listen_port
		blnd.rpc_addresses[alias] = "https://" + "127.0.0.1:" + rpc_port

		blast_lnd_log("Starting node: " + node_id)

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

		blnd.clients[n.Id] = lnrpc.NewLightningClient(client)
		node_list.Nodes[i].Address = blnd.rpc_addresses[n.Id]
	}

	time.Sleep(10 * time.Second)

	err = blnd.create_sim_ln_data(node_list, blnd.data_dir+"/sim.json")
	if err != nil {
		blast_lnd_log("Error creating simln data" + err.Error())
		return err
	}

	return nil
}

func (blnd *BlastLnd) load_nodes(simname string) error {
	blast_lnd_log("Loading nodes")

	// Untar the saved sim to blast data dir
	tar_file := blnd.home_dir + "/blast_sims/" + simname + ".tar.gz"
	file, err := os.Open(tar_file)
	if err != nil {
		return err
	}

	err = os.MkdirAll(blnd.data_dir, os.ModePerm)
	if err != nil {
		return err
	}

	err = Untar(blnd.data_dir, file)
	if err != nil {
		return err
	}

	shutdownInterceptor, err := signal.Intercept()
	if err != nil {
		blast_lnd_log("Could not set up shutdown interceptor.")
		return err
	}

	files, err := os.ReadDir(blnd.data_dir)
	if err != nil {
		return err
	}

	var loaded_nodes []LoadedNode
	saved_ports = make(map[string]*net.Listener)
	node_list := SimJsonFile{Nodes: []SimLnNode{}}
	for _, file := range files {
		if !file.IsDir() {
			continue
		}
		name := file.Name()
		lnddir := blnd.data_dir + "/" + name + "/"
		alias := "blast-" + string(name[len(name)-4:])
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
		rpc_port := loadedConfig.RawRPCListeners[0][10:]
		listen_port := loadedConfig.RawListeners[0]
		blnd.listen_addresses[alias] = "localhost:" + listen_port
		blnd.rpc_addresses[alias] = "https://" + "127.0.0.1:" + rpc_port
		n := SimLnNode{Id: alias, Address: "localhost:" + rpc_port, Macaroon: "/home/admin.macaroon", Cert: blnd.data_dir + "/" + name + "/tls.cert"}
		node_list.Nodes = append(node_list.Nodes, n)
		ln := LoadedNode{alias: alias, loadedConfig: loadedConfig, implCfg: implCfg, shutdownInterceptor: shutdownInterceptor, listen_port: listen_port, rpc_port: rpc_port}
		loaded_nodes = append(loaded_nodes, ln)

		// bind to all ports that we will need
		save_port(listen_port)
		save_port(rpc_port)
	}

	for i, n := range loaded_nodes {
		blast_lnd_log("Starting node: " + n.alias)

		free_port(n.listen_port)
		free_port(n.rpc_port)

		blnd.wg.Add(1)
		go start_lnd(n.loadedConfig, n.implCfg, n.shutdownInterceptor, blnd.wg)
		if i%10 == 0 {
			time.Sleep(10 * time.Second)
		}
	}

	simfile := blnd.data_dir + "/" + "sim.json"
	jsonFile, err := os.Open(simfile)
	if err != nil {
		return err
	}
	jsonData, _ := io.ReadAll(jsonFile)
	blnd.simln_data = jsonData

	for _, n := range node_list.Nodes {
		blast_lnd_log(n.Id)
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
		blast_lnd_log(n.Address)
		client, err := grpc.Dial(n.Address, opts...)
		if err != nil {
			blast_lnd_log("Error connecting to node: " + n.Id)
			continue
		}

		blnd.clients[n.Id] = lnrpc.NewLightningClient(client)
	}

	time.Sleep(10 * time.Second)

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

func save_port(port string) {
	listener, err := net.Listen("tcp", "localhost:"+port)
	if err != nil {
		fmt.Println("Error starting TCP server:", err)
		return
	}

	saved_ports[port] = &listener
}

func free_port(port string) {
	listener := *saved_ports[port]
	if err := listener.Close(); err != nil {
		fmt.Println("Error closing the listener:", err)
	}
}

func get_ports(used []int) ([]int, string, string) {
	listen, _ := freeport.GetFreePort()
	for slices.Contains(used, listen) {
		listen, _ = freeport.GetFreePort()
	}
	listen_port := strconv.Itoa(listen)

	rpc, _ := freeport.GetFreePort()
	for slices.Contains(used, rpc) {
		rpc, _ = freeport.GetFreePort()
	}
	rpc_port := strconv.Itoa(rpc)

	used = append(used, listen, rpc)

	return used, listen_port, rpc_port
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

func write_config(num string, currentdir string, dir string, listen_port string, rpc_port string, alias string) error {
	filePath := dir + "lnd.conf"
	conf := fmt.Sprintf(string(configfile), num, currentdir, listen_port, rpc_port, alias)

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

// Tar takes a source and variable writers and walks 'source' writing each file
// found to the tar writer; the purpose for accepting multiple writers is to allow
// for multiple outputs (for example a file, or md5 hash)
func Tar(src string, writers ...io.Writer) error {
	// ensure the src actually exists before trying to tar it
	if _, err := os.Stat(src); err != nil {
		return fmt.Errorf("unable to tar files - %v", err.Error())
	}

	mw := io.MultiWriter(writers...)

	gzw := gzip.NewWriter(mw)
	defer gzw.Close()

	tw := tar.NewWriter(gzw)
	defer tw.Close()

	// walk path
	return filepath.Walk(src, func(file string, fi os.FileInfo, err error) error {

		// return on any error
		if err != nil {
			return err
		}

		// return on non-regular files (thanks to [kumo](https://medium.com/@komuw/just-like-you-did-fbdd7df829d3) for this suggested update)
		if !fi.Mode().IsRegular() {
			return nil
		}

		// create a new dir/file header
		header, err := tar.FileInfoHeader(fi, fi.Name())
		if err != nil {
			return err
		}

		// update the name to correctly reflect the desired destination when untaring
		header.Name = strings.TrimPrefix(strings.Replace(file, src, "", -1), string(filepath.Separator))

		// write the header
		if err := tw.WriteHeader(header); err != nil {
			return err
		}

		// open files for taring
		f, err := os.Open(file)
		if err != nil {
			return err
		}

		// copy file data into tar writer
		if _, err := io.Copy(tw, f); err != nil {
			return err
		}

		// manually close here after each file operation; defering would cause each file close
		// to wait until all operations have completed.
		f.Close()

		return nil
	})
}

// Untar takes a destination path and a reader; a tar reader loops over the tarfile
// creating the file structure at 'dst' along the way, and writing any files
func Untar(dst string, r io.Reader) error {

	gzr, err := gzip.NewReader(r)
	if err != nil {
		return err
	}
	defer gzr.Close()

	tr := tar.NewReader(gzr)

	for {
		header, err := tr.Next()

		switch {

		// if no more files are found return
		case err == io.EOF:
			return nil

		// return any other error
		case err != nil:
			return err

		// if the header is nil, just skip it (not sure how this happens)
		case header == nil:
			continue
		}

		// the target location where the dir/file should be created
		target := filepath.Join(dst, header.Name)

		// the following switch could also be done using fi.Mode(), not sure if there
		// a benefit of using one vs. the other.
		// fi := header.FileInfo()

		// check the file type
		switch header.Typeflag {

		// if its a dir and it doesn't exist create it
		case tar.TypeDir:
			if _, err := os.Stat(target); err != nil {
				if err := os.MkdirAll(target, 0755); err != nil {
					return err
				}
			}

		// if it's a file create it
		case tar.TypeReg:
			ensure_directory_exists(target)
			f, err := os.OpenFile(target, os.O_CREATE|os.O_RDWR, os.FileMode(header.Mode))
			if err != nil {
				return err
			}

			// copy over contents
			if _, err := io.Copy(f, tr); err != nil {
				return err
			}

			// manually close here after each file operation; defering would cause each file close
			// to wait until all operations have completed.
			f.Close()
		}
	}
}

func start_grpc_server(wg *sync.WaitGroup, blnd *BlastLnd) *grpc.Server {
	server := grpc.NewServer()
	pb.RegisterBlastRpcServer(server, &BlastRpcServer{blast_lnd: blnd, open_channels: make(map[int64]*lnrpc.ChannelPoint)})

	address := "localhost:5050"
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
