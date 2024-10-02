![alt text](https://github.com/bjohnson5/blast/blob/main/images/blast_logo.png?raw=true)

# BLAST: Big Lightning Automated Simulation Tool

- [Introduction](#introduction)
- [Proof Of Concept](#proof-of-concept)
- [Roadmap](#roadmap)
- [Repo Tour](#repo-tour)
- [Build](#build)
- [Run](#run)

# Introduction
`The Motivation.` BLAST (Big Lightning Automated Simulation Tool) is a Modeling and Simulation (M&S) framework designed specifically for the Lightning Network. The Lightning Network is a second layer protocol built on top of the Bitcoin network to allow for faster and cheaper transactions. It uses payment channels to allow smaller transactions to be safely and efficiently routed without having to commit all the transactions to the base layer blockchain. BLAST revolutionizes the testing and analysis of Lightning Network operations with its fully automated, large-scale simulation capabilities based on real Lightning node implementations. With BLAST, users can effortlessly create highly customized test networks and define a sequence of ordered events to be executed within the simulation. Once the simulation is complete, BLAST provides comprehensive statistics and data for in-depth analysis of the simulated events.

`The Value Proposition.` This project offers significant benefits to three key stakeholders in the Lightning Network ecosystem. First, software development teams working on the Lightning Network protocol and related tools can enjoy an efficient means of testing their software against realistic simulated networks. Second, security researchers can utilize BLAST to create large-scale simulations, uncover vulnerabilities, and demonstrate potential exploits, thus improving the overall security of the Lightning Network. Lastly, Lightning Network service providers can leverage BLAST to test their infrastructure, identify weaknesses, mitigate risks, and maximize profitability. 

`The Differentiator.` BLAST distinguishes itself from existing tools through its ability to model large networks. This is especially crucial for Lightning Network service providers who require extensive test networks to conduct thorough stress tests. Additionally, BLAST's automation streamlines the simulation process, eliminating manual network operations and enabling rapid development, testing, and reproducibility. Furthermore, the foundation of BLAST on real-world Lightning Network node implementations ensures a high level of accuracy. This combination of scalability, automation, and accuracy sets BLAST apart as an exceptional testing tool in the community. 

`The Innovation.` To model large networks, phase I of BLAST will be to extensively research the most widely used Lightning implementations, such as lnd, core-lightning, eclair, and LDK, and design lightweight models based on these systems. These lightweight models will accurately simulate the behavior of real implementations while optimizing resource usage for efficient scalability. The creation of node software models within BLAST facilitates automation, enabling the simulation framework to control these models through automated events and interoperability, allowing for testing between different Lightning node implementations.  In summary, BLAST empowers the Lightning Network community by offering a comprehensive and automated M&S framework. It enables large-scale simulations, provides accurate results based on real-world implementations, and enhances the efficiency of development, testing, and analysis.

# Proof Of Concept
To read more about the ideas behind BLAST and see some initial testing results using LDK, check out this proof-of-concept project: https://github.com/bjohnson5/ln-ms-framework

# Roadmap
BLAST development will all take place in this repository and will be broken down into 3 Phases. The first two phases will be reasearch efforts and the findings will be published to this repository.

![alt text](https://github.com/bjohnson5/blast/blob/main/images/roadmap.png?raw=true)

# Repo Tour
- `blast_cli` - The CLI for running blast
- `blast_core`- The core simulation library
- `blast_example` - An example of how to use the `blast_core` library (a functional test)
- `blast_models` - The node models
- `blast_proto` - The BLAST RPC protocol definition
- `blast_web` - The web interface for running blast
- `design` - Design documents
- `images` - Graphics for the repository
- `progress_reports` - 90 day status reports on the progress and future work
- `research` - Data collected during LN implementation research

# Build
### Install bitcoind
```bash
wget https://bitcoin.org/bin/bitcoin-core-25.0/bitcoin-25.0-x86_64-linux-gnu.tar.gz
tar xzf bitcoin-25.0-x86_64-linux-gnu.tar.gz
sudo install -m 0755 -o root -g root -t /usr/local/bin bitcoin-25.0/bin/*
```

### Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Install Go
```bash
wget https://dl.google.com/go/go1.22.6.linux-amd64.tar.gz
sudo rm -rf /usr/local/go && sudo tar -C /usr/local -xzf go1.22.6.linux-amd64.tar.gz
export PATH=$PATH:/usr/local/go/bin
```

### Install Protobuf
```bash
sudo apt install -y protobuf-compiler
go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.33.0
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.3.0
export PATH=$PATH:$HOME/go/bin
```

### Install blast
```bash
git clone git@github.com:bjohnson5/blast.git
cd blast
./build.sh
```

# Run
### Run CLI
> There is no CLI yet. This is only a testing application right now, it will run some hard coded blast operations.
```bash
./run_cli.sh
```

### Run Web Application
> There is no web application yet. This is an empty project right now.
```bash
./run_web.sh
```
### Run Example Simulation
> To run the hard coded test simulation from scratch:
```bash
./run_example.sh
```

> After running the example simulation 2 simulations will be saved (`test1` and `test2`). To run these test simulations again:
```bash
./run_example.sh test1
./run_example.sh test2
```
