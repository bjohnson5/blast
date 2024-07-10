# BLAST OpenSats Progress Report #2
April 15, 2024 - July 15, 2024

## Designed and Developed BLAST RPC Protocol
BLAST is designed with an architecture that utilizes `gRPC` to remotely control external applications referred to as `models`. This approach allows for seamless integration and interoperability, enabling developers to independently create and implement these models as long as they adhere to the defined RPC endpoints. By standardizing communication through `gRPC`, BLAST gains the ability to interact with various models consistently and efficiently, regardless of their specific implementations.

During this phase of development, significant progress has been made towards implementing `gRPC` for remote communication between BLAST and the lightning node models. The `gRPC` interface was defined and an RPC server was implemented on the existing LND model, establishing the foundation for remote control capabilities. In addition, client connection functionality was integrated into the core library of BLAST. This allows the core library to initiate and manage the lifecycle of the model, starting it up as needed and establishing a connection to its RPC server. This enables the core library to execute operations on the model during a simulation run. Here are the current RPC functions:

| RPC Name           | Description                                                       |
|------------------  | ----------------------------------------------------------------- |
| `StartNodes`       | Tells a model to start a given number of nodes of its type.       |
| `GetSimLn`         | Get the SimLN data from a model.                                  |
| `GetPubKey`        | Gets the public key of a node running under this model.           |
| `ListPeers`        | List the peers of a node running under this model.                |
| `WalletBalance`    | Get the wallet balance of a node running under this model.        |
| `ChannelBalance`   | Get the channel balance of a node running under this model.       |
| `ListChannels`     | List the open channels of a node running under this model.        |
| `OpenChannel`      | Open a channel from a node running under this model.              |
| `CloseChannel`     | Close a channel from a node running under this model.             |
| `ConnectPeer`      | Connect to a peer from a node running under this model.           |
| `DisconnectPeer`   | Disconnect from a peer from a node running under this model.      |
| `GetBtcAddress`    | Get an on-chain address of a node running under this model.       |
| `GetListenAddress` | Get the connection address of a node running under this model.    |
| `StopModel`        | Shutdown the model and all of the nodes running under this model. |
| `Load`             | Load a previously saved model state.                              |
| `Save`             | Save a model state.                                               |

This table provides a clear overview of the RPC interface, listing common operations that can be performed remotely on models using the defined RPC endpoints. All of these operations can be used before, during or after a simulation is run. This lets a user configure the models/nodes, run automated events during the simulation, and then check the status of nodes after the run.

The RPC interface plays a crucial role in enabling BLAST to integrate and control different model implementations consistently. By defining a standardized set of RPC endpoints, BLAST establishes a uniform method for communication and interaction with diverse models, regardless of their specific interfaces or connection methods. This approach abstracts away the complexities of individual model implementations, allowing the tool to initiate, manage, and control models through a common interface. Models can vary widely in their internal workings and communication protocols, but as long as they adhere to the defined RPC interface, BLAST can seamlessly orchestrate their operations. 

This consistency not only simplifies the integration process but also enhances flexibility, scalability, and maintainability of the overall system. It empowers developers to focus on enhancing the capabilities of their models independently, knowing they can easily integrate them into BLAST’s ecosystem without extensive modifications. For example, a CLN model and an LDK model could both be added to BLAST and each implement the BLAST RPC functions. Those functions can then make the appropriate calls the CLN and LDK respectively.

Note that users can still manually connect full lightning nodes to the simulation network and control those nodes manually without writing the BLAST RPC interface for those models. Only nodes that the user wants BLAST to automatically control during the simulation and be able to configure through the BLAST UI need to implement the RPC interface.

For more details on the overall design of BLAST see the [design](https://github.com/bjohnson5/blast/blob/main/design/blast_design.md) documentation.

### Interesting Commits
- [c6901afe6071f2dfb6001d79c3a7c2c233b48f8f](https://github.com/bjohnson5/blast/commit/c6901afe6071f2dfb6001d79c3a7c2c233b48f8f)
- [57b9aee4ce5872c4b7e4c76786ccff61c016c1c0](https://github.com/bjohnson5/blast/commit/57b9aee4ce5872c4b7e4c76786ccff61c016c1c0)
- [9e10c2f8aad65bd8d55dee1f0593f4cb3cad3df6](https://github.com/bjohnson5/blast/commit/9e10c2f8aad65bd8d55dee1f0593f4cb3cad3df6)

## Added Automated Events
BLAST now enables the user to define events that occur at different times throughout the simulation. These events, including `StartNodeEvent`, `StopNodeEvent`, `OpenChannelEvent`, `CloseChannelEvent`, and `OnChainTransaction`, are configurable by the user before initiating the simulation. Each event triggers specific actions on the simulated network nodes, influencing the success or failure of payments. As part of the implementation, these events are processed concurrently in a separate thread to ensure they occur independently from the payment activities, mimicking real-world scenarios more accurately. This design not only introduces realism into the simulation but also allows for extensibility, as additional events can easily be incorporated in the future to simulate a wider range of network behaviors and conditions.

### Interesting Commits
- [20bc00fb313fb885acb07add692547d3d9f479fd](https://github.com/bjohnson5/blast/commit/20bc00fb313fb885acb07add692547d3d9f479fd)
- [8dd954594726b3f2afc46a3fd47f69e180be69a9](https://github.com/bjohnson5/blast/commit/8dd954594726b3f2afc46a3fd47f69e180be69a9)

## Integrated SimLN
At a high level the development of BLAST has involved two major steps so far:

1. Build a lightweight, efficient model of LND.
    - This allows the user to create a simulated lightning network with a lot of LND nodes
2. Allow that model to be controlled automatically with BLAST using a consistent RPC interface.
    - This allows the user create events that can occur during the simulation (open/close channels, fund nodes, connect to peers)

The third step is creating lightning network transactions. Once the network is up, nodes connected, channels open and events enabled, BLAST needs some network traffic. The nodes need to start paying each other and recording successful and failed payments. This is where SimLN comes into the picture. After becoming involved with the SimLN project and contributing to it a few times it became clear that this project can perfectly integrate with BLAST and provide the transaction generation that is needed. BLAST handles creating a large network and controlling the nodes and SimLN creates the payments.

Integrating SimLN into BLAST was relatively simple due to the fact that both projects are written in Rust and SimLN provides a library called `sim-lib` that can be used to create SimLN simulations. SimLN takes in json data that includes the connection details for all the nodes in the simulation along with the payment activity for the simulation. When a model starts all of its nodes it creates the SimLN json data for those nodes and returns it back to BLAST. BLAST will compile all of the SimLN json data from the different models into one input file for `sim-lib` to use. The user can also use BLAST to add specific payment activity to the simulation and that will also be included in the compiled SimLN json data that is passed to `sim-lib`. Here is a sequence diagram that shows how BLAST works with SimLN.

``` mermaid
sequenceDiagram
    blast_core->>blast_model_manager: start_model(model_name)
    blast_core->>blast_model_manager: start_nodes(model_name, num_nodes)
    blast_model_manager->>model: start_nodes()
    blast_model_manager->>model: get_sim_ln()
    model-->>blast_model_manager: simln_json_data
    blast_model_manager-->>blast_core: simln_json_data
    blast_core->>blast_simln_manager: add_nodes(simln_json_data)
    blast_core->>blast_simln_manager: add_activity("nodeid1", "nodeid2", start, count, interval, amount)
    blast_core->>blast_simln_manager: setup_simln()
    blast_simln_manager->>sim-lib: Simulation::new()
```

SimLN also writes out to a csv file the results of each payment so that user can see the results of the simulation after it is complete.

See the [testing](https://github.com/bjohnson5/blast/tree/main/research/phase2) that was done on reproducible simulations for an example of how the SimLN payment results change as different events occur during the simulation.

### Interesting Commits
- [19670281cb59d452d0c7b0370c065878f2f2936c](https://github.com/bjohnson5/blast/commit/19670281cb59d452d0c7b0370c065878f2f2936c)
- [4c2d125dd1b2bca95d4a9036a6c8278133137f06](https://github.com/bjohnson5/blast/commit/4c2d125dd1b2bca95d4a9036a6c8278133137f06)

## Reproducible Simulations
The ultimate goal of BLAST is to allow users to create large scale test networks that can be automated and easily reproduced. To enable this, functionality was added to BLAST that creates a way of saving a test network state and quickly loading that state back so that the same simulation can be repeated. For starters, BLAST will save off the bitcoind and lightning data directories so that when nodes are started up they load the saved network state. This is a simple way to get some basic save/load functionality in BLAST. The core BLAST library will handle saving the bitcoind state, events, and payment activity and each model will be responsible for saving its state when it gets a `BlastSaveRequest` from the core framework.

Here are the things that will be saved off and loaded back to enable simulation reproducibility:

- Lightning data directories for all nodes (saved by the model controlling those nodes)
- The bitcoind data directory
- The SimLN data (node connection details and payment activity)
- The BLAST simulation events

By saving these things and loading them at a later time a simulation can be run as many times as neccessary and will produce the same results. This can be seen using the `blast_example` application. The `blast_example` application starts up a network of LND nodes, funds the nodes, opens channels, connects to peers, adds events and payments and then saves the state of the network. After saving, the simulation is run. During the run the node balances, channels, peers, etc will change. However, then the `blast_example` can be run again and load the saved state and it will produce the same results as the first run.

To run the `blast_example` application:

```bash
./run_example.sh
```

For a full write up on the Load/Save functionality see the readme [here](https://github.com/bjohnson5/blast/tree/main/research/phase2).

### Interesting Commits
- [a4a8a72c511e94036cca973f9b70c0b580e9650b](https://github.com/bjohnson5/blast/commit/a4a8a72c511e94036cca973f9b70c0b580e9650b)
- [01bc0bc08f0c4c446f7c6f590990f0ee4c7bdac2](https://github.com/bjohnson5/blast/commit/01bc0bc08f0c4c446f7c6f590990f0ee4c7bdac2)
- [7a930c5f803b5a820d7c4c386b1d6b51c09f299d](https://github.com/bjohnson5/blast/commit/7a930c5f803b5a820d7c4c386b1d6b51c09f299d)
- [eb985b885c44a9c8e08b83a388344c2dc737c792](https://github.com/bjohnson5/blast/commit/eb985b885c44a9c8e08b83a388344c2dc737c792)

## SimLN Contribution: High CPU Usage Bug
- [Issue #182](https://github.com/bitcoin-dev-project/sim-ln/issues/182)
- [PR #183](https://github.com/bitcoin-dev-project/sim-ln/pull/183)

Description:
```
When sim-ln is waiting until the next payment, the CPU usage jumps to 100%.

I believe this can be traced back to the produce_simulation_results function. It contains a loop with a tokio::select! statement that has 3 branches.

The set.join_next() function documentation says it "Returns None if the set is empty." When sim-ln is not processing payments the set will be empty, so the join_next() function will continuously return None.

The tokio::select! docs say "Waits on multiple concurrent branches, returning when the first branch completes, cancelling the remaining branches."

Because the set.join_next() branch is immediately returning None, it will be the first branch to complete and the loop will iterate. This leads to a high speed loop always running while sim-ln waits for another payment.
```

This is bug that was caught due to integrating SimLN into BLAST. While running SimLN inside of BLAST the simulations were causing the CPU to jump to 100%. Because of the profiling and monitoring of BLAST performance it became clear this had to be because of the recent integration of SimLN into BLAST.

## SimLN Contribution: ValueOrRange Serialization Bug
- [Issue #187](https://github.com/bitcoin-dev-project/sim-ln/issues/187)
- [PR #188](https://github.com/bitcoin-dev-project/sim-ln/pull/188)

Description:
```
Serializing a ValueOrRange type to json turns the object into a string. Then when deserializing the sim.json file you get this error:

data did not match any variant of untagged enum ValueOrRange

This is really only an issue if you are using simln as a library and creating simulations directly in rust (not loading sim.json files)

To Reproduce

    Use sim-lib to create a simulation with a list of ActivityParser types
    Use serde_json::to_string to serialize the list to json
    Write the json data to a file so that it can be used as a sim.json for in a later simulation
    Try to load the saved sim.json file and get the error

I believe this is due to the custom serializer that simply uses serialize_str to serialize the ValueOrRange
```

The `ValueOrRange` type allows for a simple value or a range to be used in the simulation configuration file for SimLN. This config file is a json file that is deserialzed by SimLN when running a simulation. The user defines all of the nodes and activity in the json file and uses that as input to the SimLN CLI tool. Deserialization on simulation load was working as expected, however, the serialization of the simulation data into a json file was not tested because that is not yet used by the SimLN project.

BLAST uses the serialzation of simulation data to save a SimLN config file that can be loaded later. This is part of the reproducible simulation feature that BLAST offers. Users can save a simulation and then load it back at a later time to run the same simulation again. When saving a simulation BLAST was attempting to serialize the SimLN data into a json file and it was not correctly serializing the attributes that were of type `ValueOrRange`. When attempting to load that simulation back into BLAST the deserialization of the saved SimLN json failed.

A new serializer for `ValueOrRange` was written that correctly serializes the data into json and it was tested using BLAST.

## SimLN Contribution: Pull Request Reviews
After making several contributions to the SimLN project and working with the maintainers of that repository I have started getting involved in the peer review process. I helped review several pull requests by providing testing, code review and working with the author to ensure high quality code changes were made.

[PR #153](https://github.com/bitcoin-dev-project/sim-ln/pull/153)

Description:
```
Values in activities can now be fixed values (e.g. 1000) or ranges (e.g. [500, 1000]).

If a range is provided, the value will be sampled uniformly at random from it.
```

I performed several tests on this PR and found a small bug documented [here](https://github.com/bitcoin-dev-project/sim-ln/pull/153#discussion_r1566017529). I provided some feedback on code cleanup and tracked this PR until it was merged. I also used this version of SimLN in BLAST for a few days to make sure it would integrate with my project successfully.

[PR #178](https://github.com/bitcoin-dev-project/sim-ln/pull/178)

Description:
```
Adds functionality to run the simulator with an optional fixed seed. If present, the seed allows the random activity generator to run deterministically.
```

Tested and reviewed this PR along with others to ensure it was working as expected. I also used this version of SimLN in BLAST for a few days to make sure it would integrate with my project successfully. This feature will be helpful when using SimLN in BLAST because BLAST strives to create repeatable simulations. If a user wanted to use random payment activity and then saves the simulation, it would not be possible to recreate that same simulation when they load it. However, with this fixed seed functionality built into SimLN, BLAST can save the seed along with the other simulation data and a simulation can be reproduced.

[PR #190](https://github.com/bitcoin-dev-project/sim-ln/pull/190)

Description:
```
As is, we'll run with a 0 second start delay for:

    Random activities
    Defined activities with no start_secs specified

This very likely isn't the intent for defined activities (people should set start_secs=0 if they want their payments to fire immediately), and doesn't work for random activity because we just fire all payments on start.

Instead, we switch over to using our payment wait as the start delay in the absence of this field being set for defined activities and always for random activities.
```

I was involved in this PR review because it was related to a feature that I added to SimLN earlier this year. I was the author of PR [#173](https://github.com/bitcoin-dev-project/sim-ln/pull/173) and this was a follow on to that.

## Up Next
Phase III: Integrate the results of Phase I and Phase II into the BLAST tool.

Phase III is the longest phase outlined in the development plan. Over the remainder of the 2024 calendar year the development will be focused on usability. So far I have created a core rust library for creating and running simulations, an LND model that can connect to that library and an RPC protocol that will allow the core library to control the models. Now it is time to build the user interfaces that will use this core library and make it as simple as possible for users to spin up large, automated, accurate lightning network simulations. At a high level here are a few of the remaining tasks:

- Create a robust CLI that will make BLAST easy to use.
- Create a Web UI front end that will make BLAST easy to use.
- Build in functionality that will let a user have finer control over configuring a simulation.
- Implement more automated events that can be added to the simulation.

In addition to further developing the BLAST tool several DevOps process will be put in place. For example:

- Locking down the main branch and start working off of feature branches and creating Pull Requests
- Add testing
- Add a CI/CD pipeline
- Add more user documentation

## What was the OpenSats funding used for?
The grant money was used as part of my income in order to pay for living expenses.
