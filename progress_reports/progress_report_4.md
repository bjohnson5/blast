# BLAST OpenSats Progress Report #4
October 15, 2024 - January 15, 2025

## LDK and CLN Support
After getting most of the BLAST functionality built, adding a UI, and developing simulation events and activity, it became necessary to add support for more node implementations to allow for interoperability testing. In this phase, I successfully added support for LDK and CLN to BLAST, significantly enhancing its versatility and testing capabilities. This involved integrating the new node APIs into the simulation framework, ensuring compatibility with existing workflows, and enabling seamless interaction between different implementations within the simulated environment. Key tasks included implementing protocol-specific logic, adapting configuration and initialization processes, and extending the software's data-handling mechanisms to accommodate the new nodes' requirements. Extensive testing was conducted to verify the accuracy of simulated interactions and ensure stable performance under various scenarios. Build and install instructions were added to the repository along with all the necessary build scripts that will let a user clone the repository and quickly build the BLAST framework as well as three different node models (LDN, CLN, LDK). This was a challenge given that all three node implementations are written in different languages and have different dependencies. BLAST simplifies this through its build system and well documented install instructions. These additions expand the tool's applicability, allowing for more comprehensive testing and analysis of cross-implementation interoperability within the Lightning Network ecosystem.

### LDK
LDK support was added using the [ldk-node](https://github.com/lightningdevkit/ldk-node) project. This project is a Rust library that implements the full LN protocol and is a ready to use node. BLAST uses this library to create the `blast_ldk` model. The `blast_ldk` model can start up multiple `ldk-node` objects all in the same process, which saves on resources and allows BLAST to create test networks with lots of LDK nodes. The model implements the BLAST RPC interface so that the simulation framework can control all of the individual LDK nodes. This model was designed in a similar fashion as the `blast-lnd` model. Both the `blast_ldk` and `blast_lnd` models can run many individual nodes in one process. The model is responsible for managing all of the child nodes and passing the RPC calls from the simulation through to the nodes. This architecture is what lets BLAST model large Lightning Networks.

> NOTE: The `ldk-node` project does not currently have an RPC interface. This is ok when using it in BLAST because the BLAST model can control the nodes directly, however SimLN requires that nodes have an RPC connection in order to generate payments to and from it. Because of this limitation, the LDK nodes can only be used as forwarding nodes in a BLAST simulation and not a source or destination node. The LDK nodes are useful in that they can create multiple routes to and from other nodes, help forward payments, and fill out a big LN. However, more work is needed to implement a direct RPC to each node so that SimLN can use them as source and destination nodes for its payment activity.

### CLN
The CLN support was a little trickier than the LND and LDK support because CLN is a more traditional daemon application with several sub-processes and was designed to only have one node running at a time. This means that it uses more resources and multiple processes must be launched when starting multiple nodes. This was achieved by separating the data directories, using different sockets, and carefully writing the startup and shutdown routines. BLAST users can now start multiple CLN nodes, however the number of CLN nodes that can be added to a simulation is more limited than LND and LDK. CLN nodes do come with an RPC interface, therefore they can be more easily integrated into BLAST and SimLN and can be used as source, destination or forwarding nodes in a simulation.

### Interesting Commits
- [LDK Model PR #13](https://github.com/bjohnson5/blast/pull/13)
- [CLN Model PR #15](https://github.com/bjohnson5/blast/pull/15)

## SimLN Contributions
I continued working with the SimLN team by assisting in PR reviews as well as opening and fixing an issue I discovered while using SimLN in BLAST.

### PR Review
SimLN PR #201 helped improve channel validation by checking some values when starting up the `sim-node` project. The `sim-node` project is a fully simulated node that does not contain any peer to peer functionality and it primarily used for testing. I reviewed the new validation logic and suggested an additional check to help further validate the channels.

SimLN PR #202 was a small refactor of the `Simulation` object where a new `SimulationCfg` struct was defined and used. I reviewed and pointed out one issue where the simulation results object should have been left in the `Simulation` struct and not moved to `SimulationCfg`.

### CLN Feature Flags Issue
While adding CLN support to BLAST I noticed that SimLN was not able to send keysend payments from a CLN node in the BLAST simulation. Even though the CLN nodes in BLAST had keysend enabled in their feature bits, SimLN was throwing an error saying that keysend was not enabled on the node. I started doing some testing directly on SimLN and discovered the issue was caused by SimLN not parsing the feature flags correctly in one particular place. Below is the bug report:

- Describe the bug
When I attempt to use a CLN node as a destination in the defined activity, the simulation does not start and gives the error:

```
ValidationError("Destination node does not support keysend, cln-0000(0287eb...c97d33)"
```

However, CLN appears to have keysend enabled and it can send keysend payments when it is used as the source node in the defined activity.

- To Reproduce
Configure a simple defined activity with an LND node as the source and a CLN node as the destination.

- Possible Solution
I believe this is due to the feature flags not being parsed correctly when creating a CLN node in SimLN.

In the `get_node_info` function in cln.rs the feature bits are reversed before calling `NodeFeatures::from_le_bytes`
```
if let Some(node) = nodes.pop() {
    Ok(NodeInfo {
        pubkey: *node_id,
        alias: node.alias.unwrap_or(String::new()),
        features: node
            .features
            .clone()
            .map_or(NodeFeatures::empty(), |mut f| {
                // We need to reverse this given it has the CLN wire encoding which is BE
                f.reverse();
                NodeFeatures::from_le_bytes(f)
            }),
    })
}
```

But in the `new` function for `ClnNode` the feature bits are passed directly into `NodeFeatures::from_le_bytes`
```
let features = if let Some(features) = our_features {
    NodeFeatures::from_le_bytes(features.node)
} else {
    NodeFeatures::empty()
};

Ok(Self {
    client,
    info: NodeInfo {
        pubkey,
        features,
        alias,
    },
})
```

I submitted PR #209 as a fix for this issue and it was merged.

### Interesting Commits
- [SimLN PR #201](https://github.com/bitcoin-dev-project/sim-ln/pull/201#discussion_r1840744060)
- [SimLN PR #202](https://github.com/bitcoin-dev-project/sim-ln/pull/202#discussion_r1841051351)
- [SimLN Issue #208](https://github.com/bitcoin-dev-project/sim-ln/issues/208)
- [SimLN PR #209](https://github.com/bitcoin-dev-project/sim-ln/pull/209)

## Cleanup
As the final phase of BLAST development was coming to a close, I spent time testing large simulations and all the functionality of BLAST. I tried to cleanup comments, documentation, and code as well as fix any bugs that I found in testing. A summary of the changes made in this testing cycle can be seen in PR #18.

### Interesting Commits
- [Clean up PR #18](https://github.com/bjohnson5/blast/pull/18)

## ChainCode Research Day
On November 22, 2024 I attended the Chaincode research day in New York City. I had the chance to meet with other open source developers, hear about new ideas in bitcoin and lightning, and talk to people about BLAST and LN simulation. Chaincode did a great job of putting an event together with both academic researchers and engineers. I met several researchers that are in need of LN simulation tools to help them test their theoretical work. I plan to follow up with these researchers and collaborate with them as I continue to develop these tools.

## Test Range
In order to test BLAST, SimLN, and several of the LN node implementations I created a new repository called [ln-test-range](https://github.com/bjohnson5/ln-test-range). This is a place where I plan to begin writing large scale functional tests, specifically focused on interoperability and scaling. So far, I have set up two test cases using BLAST that can be run and then analyzed to learn about how the nodes behave in a given configuration.

## Up Next
1.  Create a web UI for easier simulation management and usability. Create a web server that will allow users to use BLAST from a browser. This will help with deployment and let BLAST be used within a docker container on operating systems other than linux.
2.  More testing. BLAST still requires a lot more testing and contains several known bugs. Most   of the known issues have been documented [here](https://github.com/bjohnson5/blast/issues).
3.  Establish an official release. Package the BLAST tool and the models in a way that is easy to download and install as an official release.

## What was the OpenSats funding used for?
The grant money was used as part of my income in order to pay for living expenses.
