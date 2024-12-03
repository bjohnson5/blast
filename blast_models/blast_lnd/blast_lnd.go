package main

import (
	"archive/tar"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
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

// The config file for all lnd nodes (lnd.conf)
const CONFIG_FILE string = `listen=%[3]s
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

// The name of this model (should match the name in model.json)
const MODEL_NAME string = "blast_lnd"

// The directory to save simulations
const SIM_DIR string = ".blast/blast_sims"

// The temporary directory to save runtime lnd data
const DATA_DIR string = ".blast/blast_data/blast_lnd"

// The Blast RPC address
const RPC_ADDR string = "localhost:5050"

// The default macaroon file
const MACAROON string = ".blast/admin.macaroon"

// The data that is stored in the sim-ln sim.json file
type SimLnNode struct {
	Id       string `json:"id"`
	Address  string `json:"address"`
	Macaroon string `json:"macaroon"`
	Cert     string `json:"cert"`
}

// The sim.json file for a sim-ln simulation
type SimJsonFile struct {
	Nodes []SimLnNode `json:"nodes"`
}

// A channel point struct that is used to close an existing channel
type ChannelPoint struct {
	Source      string
	Dest        string
	FundingTxid []byte
	OutputIndex uint32
}

// The main blast_lnd struct
type BlastLnd struct {
	clients          map[string]lnrpc.LightningClient
	listen_addresses map[string]string
	rpc_addresses    map[string]string
	simln_data       []byte
	shutdown_ch      chan struct{}
	home_dir         string
	data_dir         string
	open_channels    map[string]ChannelPoint
	wg               *sync.WaitGroup
}

// A currently loaded node
type LoadedNode struct {
	alias               string
	loadedConfig        *lnd.Config
	implCfg             *lnd.ImplementationCfg
	shutdownInterceptor signal.Interceptor
	listen_port         string
	rpc_port            string
}

// Ports that need to be saved for nodes that will be started at a later time
var saved_ports map[string]*net.Listener

func main() {
	// Set up the wait group, data directory and shutdown channel for this run
	var wg sync.WaitGroup
	shutdown_channel := make(chan struct{})
	dir, err := os.UserHomeDir()
	if err != nil {
		blast_lnd_log("Could not get home directory.")
		return
	}
	blast_data_dir := dir + "/" + DATA_DIR

	// Create the main blast_lnd struct and start the RPC server so that blast can connect to this model
	blast_lnd := BlastLnd{clients: make(map[string]lnrpc.LightningClient), listen_addresses: make(map[string]string), rpc_addresses: make(map[string]string), shutdown_ch: shutdown_channel, home_dir: dir, data_dir: blast_data_dir, open_channels: make(map[string]ChannelPoint), wg: &wg}
	server := start_grpc_server(&wg, &blast_lnd)

	// Listen for a shutdown
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

// Start a given number of nodes
func (blnd *BlastLnd) start_nodes(num_nodes int) error {
	blast_lnd_log("Starting nodes")

	// Create a shutdown interceptor, blast_lnd nodes will shutdown on ctrl-c
	shutdownInterceptor, err := create_shutdown_listener()
	if err != nil {
		return err
	}

	// Create the sim-ln json data object and a list of used ports to keep track of active ports
	node_list := SimJsonFile{Nodes: []SimLnNode{}}
	used_ports := make([]int, 2*num_nodes)

	// For the given number of nodes: write an lnd.conf file and start up an lnd node
	for i := 0; i < num_nodes; i++ {
		// Set the node_id
		node_id := pad_with_zeros(i, 4)

		// Set the lnddir and alias
		lnddir, alias := blnd.get_name_lnddir(node_id)

		// Get ports for this node
		var listen_port string
		var rpc_port string
		used_ports, listen_port, rpc_port = get_ports(used_ports)

		// Write the lnd.conf file to the lnd data dir
		err := write_config(node_id, blnd.data_dir, lnddir, listen_port, rpc_port, alias)
		if err != nil {
			blast_lnd_log("Error writing lnd config to file: " + err.Error())
			return err
		}

		// Load the lnd configuration
		loadedConfig, implCfg, err := load_lnd_config(shutdownInterceptor, lnddir)
		if err != nil {
			return err
		}

		homeDir, err := os.UserHomeDir()
		if err != nil {
			return err
		}

		mac := homeDir + "/" + MACAROON

		// Save off important information about this node
		n := SimLnNode{Id: alias, Address: "localhost:" + rpc_port, Macaroon: mac, Cert: blnd.data_dir + "/lnd" + node_id + "/tls.cert"}
		node_list.Nodes = append(node_list.Nodes, n)
		blnd.listen_addresses[alias] = "localhost:" + listen_port
		blnd.rpc_addresses[alias] = "https://" + "127.0.0.1:" + rpc_port

		// Start the node
		blast_lnd_log("Starting node: " + alias)
		blnd.wg.Add(1)
		go start_lnd(loadedConfig, implCfg, shutdownInterceptor, blnd.wg)

		// After starting 10 nodes, wait some time to let the nodes get up and running
		if i%10 == 0 {
			time.Sleep(10 * time.Second)
		}
	}

	// After starting all of the nodes, attempt to connect the model to each running lnd node
	blnd.connect_to_nodes(node_list.Nodes)
	time.Sleep(10 * time.Second)

	// After successfully connecting to all nodes, create the sim-ln data for all the running nodes
	err = blnd.create_sim_ln_data(node_list, blnd.data_dir+"/sim.json")
	if err != nil {
		blast_lnd_log("Error creating simln data" + err.Error())
		return err
	}

	return nil
}

// Load previously saved nodes
func (blnd *BlastLnd) load_nodes(path string) error {
	blast_lnd_log("Loading nodes")

	// Create a shutdown interceptor, blast_lnd nodes will shutdown on ctrl-c
	shutdownInterceptor, err := create_shutdown_listener()
	if err != nil {
		return err
	}

	// Untar the saved sim to blast data dir
	file, err := os.Open(path)
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

	// Get all the nodes in the data dir
	files, err := os.ReadDir(blnd.data_dir)
	if err != nil {
		return err
	}

	// Create the sim-ln json data object and a list of saved ports to keep track of needed ports
	var loaded_nodes []LoadedNode
	saved_ports = make(map[string]*net.Listener)
	node_list := SimJsonFile{Nodes: []SimLnNode{}}

	// For all the nodes: load their config file and start them
	for _, file := range files {
		// Process the current directory
		if !file.IsDir() {
			continue
		}
		name := file.Name()

		// Set the node_id
		node_id := string(name[len(name)-4:])

		// Set the lnddir and alias
		lnddir, alias := blnd.get_name_lnddir(node_id)

		// Load the lnd configuration
		loadedConfig, implCfg, err := load_lnd_config(shutdownInterceptor, lnddir)
		if err != nil {
			return err
		}

		homeDir, err := os.UserHomeDir()
		if err != nil {
			return err
		}

		mac := homeDir + "/" + MACAROON

		// Save off important information about this node
		rpc_port := loadedConfig.RawRPCListeners[0][10:]
		listen_port := loadedConfig.RawListeners[0]
		blnd.listen_addresses[alias] = "localhost:" + listen_port
		blnd.rpc_addresses[alias] = "https://" + "127.0.0.1:" + rpc_port
		n := SimLnNode{Id: alias, Address: "localhost:" + rpc_port, Macaroon: mac, Cert: blnd.data_dir + "/" + name + "/tls.cert"}
		node_list.Nodes = append(node_list.Nodes, n)
		ln := LoadedNode{alias: alias, loadedConfig: loadedConfig, implCfg: implCfg, shutdownInterceptor: shutdownInterceptor, listen_port: listen_port, rpc_port: rpc_port}
		loaded_nodes = append(loaded_nodes, ln)

		// bind to all ports that we will need
		save_port(listen_port)
		save_port(rpc_port)
	}

	// Start the nodes
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

	// Load the saved sim.json file and set the simln_data
	simfile := blnd.data_dir + "/" + "sim.json"
	jsonFile, err := os.Open(simfile)
	if err != nil {
		return err
	}
	jsonData, _ := io.ReadAll(jsonFile)
	blnd.simln_data = jsonData

	// After starting all of the nodes, attempt to connect the model to each running lnd node
	blnd.connect_to_nodes(node_list.Nodes)
	time.Sleep(10 * time.Second)

	return nil
}

// Connect to each of the nodes RPC server so that this model can control the nodes
func (blnd *BlastLnd) connect_to_nodes(nodes []SimLnNode) {
	for i, n := range nodes {
		// Set up the tls creds and options
		var tlsCreds credentials.TransportCredentials
		tlsCreds, err := credentials.NewClientTLSFromFile(n.Cert, "")
		if err != nil {
			blast_lnd_log("Error reading TLS cert" + err.Error())
			continue
		}

		opts := []grpc.DialOption{
			grpc.WithBlock(),
			grpc.WithTransportCredentials(tlsCreds),
		}

		// Connect to the server
		client, err := grpc.Dial(n.Address, opts...)
		if err != nil {
			blast_lnd_log("Error connecting to node: " + n.Id)
			continue
		}

		// Save the RPC client so that it can be used later
		blnd.clients[n.Id] = lnrpc.NewLightningClient(client)

		// Change the address to the sim-ln format after connecting
		nodes[i].Address = blnd.rpc_addresses[n.Id]
	}
}

// Create the sim-ln formatted data for all the nodes that this model controls
func (blnd *BlastLnd) create_sim_ln_data(obj interface{}, filename string) error {
	// Serialize the data to json
	jsonData, err := json.MarshalIndent(obj, "", "    ")
	if err != nil {
		return err
	}

	// Create and write to the file
	file, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer file.Close()

	_, err = file.Write(jsonData)
	if err != nil {
		return err
	}

	// Save the raw json data
	blnd.simln_data = jsonData
	return nil
}

// Save the open channels so that they can still be accessed after a load
func (blnd *BlastLnd) save_channels(filename string) error {
	// Convert the map to a JSON string
	data, err := json.Marshal(blnd.open_channels)
	if err != nil {
		return err
	}

	// Write the JSON string to the file
	err = os.WriteFile(filename, data, 0644)
	if err != nil {
		return err
	}

	return nil
}

// Load the saved channels so that the model can still control those channels
func (blnd *BlastLnd) load_channels(filename string) error {
	// Read the JSON file
	data, err := os.ReadFile(filename)
	if err != nil {
		return err
	}

	// Create a map to hold the data
	m := make(map[string]ChannelPoint)

	// Unmarshal the JSON data into the map
	err = json.Unmarshal(data, &m)
	if err != nil {
		return err
	}

	blnd.open_channels = m

	return nil
}

// Create a lnddir path and name for the node based on the node id
func (blnd *BlastLnd) get_name_lnddir(id string) (string, string) {
	// Create a lnd data dir
	lnddir := blnd.data_dir + "/lnd" + id + "/"
	// Create a node name
	alias := MODEL_NAME + "-" + id

	return lnddir, alias
}

// Load the lnd config file
func load_lnd_config(shutdownInterceptor signal.Interceptor, lnddir string) (*lnd.Config, *lnd.ImplementationCfg, error) {
	lnd.DefaultLndDir = lnddir
	lnd.DefaultConfigFile = lnddir + "lnd.conf"
	loadedConfig, err := lnd.LoadConfig(shutdownInterceptor)
	if err != nil {
		if e, ok := err.(*flags.Error); !ok || e.Type != flags.ErrHelp {
			blast_lnd_log("Error loading config.")
		}
		return nil, nil, err
	}
	implCfg := loadedConfig.ImplementationConfig(shutdownInterceptor)

	return loadedConfig, implCfg, err
}

// Create the interrupt handler for lnd nodes
func create_shutdown_listener() (signal.Interceptor, error) {
	shutdownInterceptor, err := signal.Intercept()
	if err != nil {
		blast_lnd_log("Could not set up shutdown interceptor.")
		return signal.Interceptor{}, err
	}

	return shutdownInterceptor, err
}

// Bind to a port so that no other process (node) will use it
func save_port(port string) {
	listener, err := net.Listen("tcp", "localhost:"+port)
	if err != nil {
		fmt.Println("Error starting TCP server:", err)
		return
	}

	saved_ports[port] = &listener
}

// Stop listening on a saved port so that one of the model's nodes can use it
func free_port(port string) {
	listener := *saved_ports[port]
	if err := listener.Close(); err != nil {
		fmt.Println("Error closing the listener:", err)
	}
}

// Get free ports for a node to use
func get_ports(used []int) ([]int, string, string) {
	listen, _ := freeport.GetFreePort()
	for slices.Contains(used, listen) {
		listen, _ = freeport.GetFreePort()
	}
	listen_port := strconv.Itoa(listen)
	used = append(used, listen)

	rpc, _ := freeport.GetFreePort()
	for slices.Contains(used, rpc) {
		rpc, _ = freeport.GetFreePort()
	}
	rpc_port := strconv.Itoa(rpc)
	used = append(used, rpc)

	return used, listen_port, rpc_port
}

// Log a message
func blast_lnd_log(message string) {
	log.Println("[BLAST MODEL:" + MODEL_NAME + "] " + message)
}

// Create a node id in the format: 0000
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

// Write the lnd config string to a file
func write_config(num string, currentdir string, dir string, listen_port string, rpc_port string, alias string) error {
	// Create the config string
	filePath := dir + "lnd.conf"
	conf := fmt.Sprintf(string(CONFIG_FILE), num, currentdir, listen_port, rpc_port, alias)

	// Make the data directory for this node
	err := ensure_directory_exists(filePath)
	if err != nil {
		return err
	}

	// Create the lnd.conf file
	file, err := os.Create(filePath)
	if err != nil {
		return err
	}
	defer file.Close()

	// Write the string to the file
	_, err = file.WriteString(conf)
	if err != nil {
		return err
	}

	return nil
}

// See if a directory exists and create it if not
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

// Start the blast RPC server so that the blast framework can connect to this model
func start_grpc_server(wg *sync.WaitGroup, blnd *BlastLnd) *grpc.Server {
	server := grpc.NewServer()
	pb.RegisterBlastRpcServer(server, &BlastRpcServer{blast_lnd: blnd})

	listener, err := net.Listen("tcp", RPC_ADDR)
	if err != nil {
		blast_lnd_log("Failed to listen: " + err.Error())
		return nil
	}

	wg.Add(1)
	go func() {
		defer wg.Done()
		blast_lnd_log("Server started at " + RPC_ADDR)
		if err := server.Serve(listener); err != nil {
			blast_lnd_log("Failed to serve: " + err.Error())
		}
	}()

	return server
}

// Start an lnd node
func start_lnd(cfg *lnd.Config, implCfg *lnd.ImplementationCfg, interceptor signal.Interceptor, wg *sync.WaitGroup) {
	defer wg.Done()

	if err := lnd.Main(cfg, lnd.ListenerCfg{}, implCfg, interceptor); err != nil {
		blast_lnd_log("Could not start lnd: " + err.Error())
	}
}
