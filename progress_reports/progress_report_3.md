# BLAST OpenSats Progress Report #3
July 15, 2024 - October 15, 2024

## BLAST TUI
The development of a Terminal User Interface (TUI) is a key component of our larger effort to create a useful modeling and simulation tool for the Lightning Network. A TUI is a text-based interface that allows users to interact with software applications directly through a command-line interface (CLI). This interface utilizes keyboard inputs to execute commands, configure settings, and display outputs. Unlike graphical user interfaces (GUIs), which rely on visual elements like windows and icons, TUIs provide a more streamlined and efficient interaction method, especially beneficial for users who prefer keyboard navigation or operate in environments where graphical capabilities are limited.

TUIs can be developed more rapidly than full GUIs due to several factors. Firstly, the complexity of creating visual elements—such as buttons, sliders, and menus—can significantly extend development timelines. In contrast, TUIs primarily focus on text-based inputs and outputs, which simplifies the coding process. Additionally, TUIs require less design iteration since they lack the visual design considerations that GUIs demand, allowing developers to concentrate on functionality. This made the TUI an ideal first step for our project, as it provided a usable interface quickly and effectively. By implementing a TUI, we could introduce a user-friendly interaction model that enhances accessibility and prepares the groundwork for potential future GUI development.

By integrating a TUI into our modeling and simulation application, we aim to enhance accessibility for users who work in terminal environments, cater to advanced users who prefer command-line operations, and ultimately provide a more versatile tool for simulation management.

The TUI consists of four main pages, each designed to facilitate different aspects of the simulation process:

1. **New Page:** This page allows users to create new simulations by inputting the number of each node type they would like in the simulation. This will start up the nodes and then pass control to the `Configure` page where the user can further set up the network and simulation.

2. **Load Page:** Users can load pre-existing simulations from this page, streamlining the process of accessing and reusing previous work. After the simulation is loaded the TUI will switch over the `Configure` page where the user can edit the loaded simulation if they would like.

3. **Configure Page:** On this page, users can modify various simulation parameters before execution, ensuring that each simulation can be tailored to specific needs. Users can add events and payment activity, open channels, view information about the nodes, etc. When the user is content with the simulation/network they can start the simulation and this will switch the TUI to the `Run` page.

4. **Run Page:** This final page enables users to execute their simulations, providing real-time feedback and output as the simulation runs. After the simulation ends the TUI returns to the `Configure` page where the user can inspect the network, re-run the simulation, or shutdown the network and start over.

Running BLAST simulations involves two parts: the network and the simulation. The network refers to the nodes and channels that make up the regtest Lightning Network. The simulation refers to the payment activity and the events. These parts are started/stopped independently, meaning the user can start up the network and make changes to the network before starting the payments and events. After the payments and events are run the simulation will stop but the network stays up so that the user can inspect the nodes and see how the payments and events impacted the network. At this point the user can either stop the network and start over or re-run the simulation on the network. This workflow promotes quick reproducibility and flexibility for the user. The BLAST TUI helps facilitate this process.

![](../images/tui_flow.png)

See a quick demo of the TUI [here](https://github.com/bjohnson5/blast/blob/main/images/blast_cli.gif)

On the `Configure` page there is a CLI with the following commands:
| Command            | Description                                               |
|------------------  | --------------------------------------------------------- |
| `save`             | Save a simulation so that it can be loaded later.         |
| `add_activity`     | Add payment activity to the simulation.                   |
| `add_event`        | Add an event to the simulation.                           |
| `get_nodes`        | List all the node names currently running.                |
| `get_pub_key`      | Get the public key of a node.                             |
| `list_peers`       | List all the peers of a node.                             |
| `wallet_balance`   | Get the on-chain balance of a node.                       |
| `channel_balance`  | Get the off-chain blanace of a node.                      |
| `list_channels`    | List the channels of a node.                              |
| `open_channel`     | Open a channel between two nodes.                         |
| `close_channel`    | Close a channel between two nodes.                        |
| `connect_peer`     | Connect to another node.                                  |
| `disconnect_peer`  | Disconnect from another node.                             |
| `fund_node`        | Send on-chain funds to a node.                            |

### Interesting Commits
- [BLAST PR #1](https://github.com/bjohnson5/blast/pull/1)

## Runtime Statistics
Part of creating the TUI for BLAST was implementing a new feature that displays runtime statistics of simulations. On the `Run` page of the TUI BLAST will display the number of payment attempts and the success rate of those attempts. The payment statistics were already being tracked and recorded by the payment generation project SimLN, which is a dependency of BLAST. This feature required specific functions and data to be publicly exposed by the SimLN library. The following section outlines the detailed process I undertook to achieve this integration.

### Step 1: Identifying Requirements
To implement the runtime stats feature in BLAST, I conducted a thorough analysis of the necessary data points and functions required from SimLN. This included identifying specific simulation metrics, such as execution time, resource usage, and performance bottlenecks.

### Step 2: Collaboration with SimLn
Upon identifying the dependencies, I created an issue in the SimLN repository outlining the need for certain functions and data to be exposed to facilitate the integration with BLAST. This issue included detailed descriptions of the required functionalities and how they would enhance both SimLN and BLAST.

### Step 3: Implementing Changes in SimLN
After receiving feedback from the SimLN maintainers, I proceeded to implement the necessary changes. This involved adding new public functions to calculate and retrieve the required simulation metrics. Once the modifications were completed, I submitted a pull request for review. The changes were discussed with the SimLN team, and after addressing their comments and suggestions, the pull request was successfully merged into the main branch of SimLN.

### Step 4: Integrating Changes into BLAST
With the new functions and data exposed in SimLN, I updated BLAST to utilize these newly available features, allowing BLAST to fetch and display real-time simulation statistics. This integration involved:

- Modifying the existing user interface to incorporate the new statistics display.
- Ensuring that the runtime stats were accurately calculated and presented in a user-friendly manner.
- Conducting thorough testing to confirm that the integration worked seamlessly and did not introduce any bugs.

The successful integration of the runtime stats feature in BLAST was made possible through effective collaboration with the SimLN project and timely implementation of necessary changes. This enhancement not only improves the user experience for BLAST users but also strengthens the relationship between the two projects.

### Interesting Commits
- [SimLN PR #197](https://github.com/bitcoin-dev-project/sim-ln/pull/197)
- [SimLN Issue #196](https://github.com/bitcoin-dev-project/sim-ln/issues/196)
- [f0dbd114b83de0161267a8731e77a6918e378170](https://github.com/bjohnson5/blast/pull/1/commits/f0dbd114b83de0161267a8731e77a6918e378170)
- [b6f9229121a7a497bf635788713c4e3e07c0e298](https://github.com/bjohnson5/blast/pull/1/commits/b6f9229121a7a497bf635788713c4e3e07c0e298)

## LND Deadlock
During development and testing of BLAST a potential deadlock was discovered in `LND`. By creating a simulation with several nodes and channels and using SimLN to generate network traffic a mutex deadlock was found that caused an LND node to freeze up and quit processing peer messages or RPC calls. A few important points to note:

- BLAST was using `LND` nodes with the `graphCache` turned off to save memory. This is not the default configuration so this bug went unnoticed. However, if other LND users are trying to save memory they may run without the `graphCache` and are then vulnerable to this issue.
- SimLN was vital in finding this problem because it generated enough network traffic that this problem was occuring regularly. Manually sending a few transactions from one node to another would not have triggered the deadlock. It was found because the node was in the process of finding a route for its next payment and also managing incoming messages from its peers about previous transactions. This multi-tasking was caused by the automation of BLAST and SimLN and is ultimately responsible for uncovering the bug.
- The repeadablity that BLAST allows for is what enabled the troubleshooting of this bug and understanding what the underlying problem was. We started noticing the nodes freezing up during a BLAST simulation and so we saved the simulation and began running it again. Because the saved simulation executes the same way each time, we repeatedly were seeing the freezing issue. Debugging was possible because we could start and stop the simulation over and over again and observe what was happening in the LND code.

This is an excellent example of the use case for testing tools like BLAST and SimLN. Research like this will uncover edge case conditions like this particular deadlock that can then be fixed and make lightning node software more reliable and secure.

### Technical Analysis
In `lnwallet/channel.go` the `LightningChannel` struct defines several methods that the comments explain as the "state machine which corresponds to the current commitment protocol wire spec". These methods are: `SignNextCommitment`, `ReceiveNewCommitment`, `RevokeCurrentCommitment`, and `ReceiveRevocation`. Each of these will first lock the LightningChannel: `lc.lock()` and then they will typically attempt to update the channel db.

When updating the channel db, sometimes the database must be re-sized and re-mapped to memory using the `mmap` function in bbolt's db.go file. This function first attempts to lock the `mmaplock` mutex.

This is all fine except that if one of the state machine functions is called while the node is trying to find a route. In that case a deadlock could occur. The `RequestRoute` function in `payment_session.go` will get a routing graph from the db and this will acquire the `mmaplock` on the db (for good reason, it needs to be sure the db is not re-mapped while it is using it to find a route). It will eventually call functions of the `LightningChannel` struct in order to find bandwidth, balances, etc... It is possible that these functions are locked by one of the state machine methods and that state machine method could be stuck waiting on the `mmaplock`. Here are a few example call stacks that show how a deadlock can occur:

### Thread0(payment_session.go)
```c
RequestRoute
    NewGraphSession
        NewPathFindTx
            BeginReadTx
                beginTx
                    mmap.RLock // acquires the mmap read lock on this database

    pathFinder/findPath
        getOutgoingBalance
            getOutgoingBalance::cb
                availableChanBandwidth
                    getBandwidth
                    EligibleToForward
                        EligibleToUpdate
                            RemoteNextRevocation
                                lc.RLock // blocks because Thread1 has write lock
```

### Thread1(lnwallet/channel.go)
```c
// if this happens during a RequestRoute AND calls mmap it will deadlock
SignNextCommitment
    lc.Lock
    AppendRemoteCommitChain
        kvdb.Update
            ...
            beginRWTx
                rwlock.Lock
            tx.Commit
                commitFreelist
                    allocate
                        allocate
                            mmap
                                mmaplock.Lock // sometimes it makes it here and locks because Thread0 has a read lock
```

### Thread2(lnrpc/routerrpc/router_server.go)
```c
TrackPaymentV2
    subscribePayment
        SubscribePayment
            FetchPayment
                kvdb.View
                    ...
                    beginTx
                        mmap.RLock // blocks because Thread0 has a read lock and there is a waiting write lock in Thread1
```

- Thread1 is holding `lc.Lock` and waiting for `mmap.Lock`
- Thread0 is holding `mmap.RLock` and waiting for `lc.Lock`
- Thread2 is waiting on `mmap.RLock` (but another reader is not allowed until the writer has its chance)

If something happens in `lnwallet/channel.go` that locks a LightningChannel object and then needs to re-map the database (through the Update() call) while `RequestRoute` is iterating channels, there could be a deadlock. This is only happening when the `graphCache` is not used... because the channel graph db has to be remapped more often when not using the cache. If the `graphCache` is used the route finding will not have to make db operations and the mmap will not be an issue.

### Interesting Commits
- [LND Discussion #9060](https://github.com/lightningnetwork/lnd/discussions/9060)
- [LND Issue #9133](https://github.com/lightningnetwork/lnd/issues/9133)

## Up Next
Continuing Phase III: Integrate the results of Phase I and Phase II into the BLAST tool.

1.  Create a web UI for easier simulation management and usability
    - Create a web server that will allow users to use BLAST from a browser. This will help with deployment and let BLAST be used within a docker container on operating systems other than linux.
2.  Use LDK to build a super lightweight node that only handles forwarding payments.
    - This node can then be added to BLAST as another model that is available to the users
    - So far we have been exploring ways to create lightweight implementation-specific nodes that are still very accurate representations of the full implementation. We created an LND node that is just a little more efficient, but is still essentially a full LND node. A more effective strategy might be to create generic simulation nodes that simply help make up a test network and then use full implementations for the nodes of interest that the user is trying test. These "generic simulation nodes" would not be very realistic, but that does not really matter because you are not interested in how these nodes operate, they are just there to help facilitate the simulation. For example, if you are trying to test out some simulated attack on LND nodes, you could have a few full LND nodes running but you need several different routes to and from these nodes. You could start up these "generic simulation nodes" and open channels to them to create a network around the LND nodes. These nodes do not have to be explicitly defined in SimLN because they won't initiate any payments, they will just forward payments between the real LND nodes. This could increase the size of the test network that is possible.

## What was the OpenSats funding used for?
The grant money was used as part of my income in order to pay for living expenses.
