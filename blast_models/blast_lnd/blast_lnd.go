package main

import (
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"sync"
	"time"

	"github.com/jessevdk/go-flags"
	"github.com/lightningnetwork/lnd"
	"github.com/lightningnetwork/lnd/signal"
)

const configfile string = `listen=9%[1]s
rpclisten=localhost:10%[1]s
restlisten=localhost:8%[1]s
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

func main() {
	go func() {
		http.ListenAndServe("localhost:6060", nil)
	}()

	var wg sync.WaitGroup

	// Hook interceptor for os signals.
	shutdownInterceptor, err := signal.Intercept()
	if err != nil {
		_, _ = fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	for i := 0; i < 100; i++ {
		fmt.Printf("Iteration %d\n", i)

		dir, err := filepath.Abs(filepath.Dir(os.Args[0]))
		if err != nil {
			fmt.Println(err)
		}
		fmt.Println(dir)

		lnddir := dir + "/lnd" + strconv.Itoa(i) + "/"
		write_config(strconv.Itoa(i), dir, lnddir)
		// Load the configuration, and parse any command line options. This
		// function will also set up logging properly.
		lnd.DefaultLndDir = lnddir
		lnd.DefaultConfigFile = lnddir + "lnd.conf"
		loadedConfig, err := lnd.LoadConfig(shutdownInterceptor)
		if err != nil {
			if e, ok := err.(*flags.Error); !ok || e.Type != flags.ErrHelp {
				// Print error if not due to help request.
				err = fmt.Errorf("failed to load config: %w", err)
				_, _ = fmt.Fprintln(os.Stderr, err)
				os.Exit(1)
			}

			// Help was requested, exit normally.
			os.Exit(0)
		}
		implCfg := loadedConfig.ImplementationConfig(shutdownInterceptor)
		wg.Add(1)
		go start_lnd(loadedConfig, implCfg, shutdownInterceptor, &wg)

		if i%10 == 0 {
			fmt.Println("Sleeping for 10 seconds...")
			time.Sleep(10 * time.Second)
		}
	}

	wg.Wait()
}

func write_config(num string, currentdir string, dir string) {
	// Define the target file path and content
	filePath := dir + "lnd.conf"
	conf := fmt.Sprintf(string(configfile), num, currentdir)

	// Ensure the directory structure exists
	err := ensureDirectoryExists(filePath)
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

// ensureDirectoryExists creates the directory structure for the given file path
func ensureDirectoryExists(filePath string) error {
	dir := filepath.Dir(filePath)
	return os.MkdirAll(dir, os.ModePerm)
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
