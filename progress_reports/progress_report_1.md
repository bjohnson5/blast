# BLAST OpenSats Progress Report #1
January 15, 2024 - April 15, 2024

## Analyzed LND Memory Usage
Phase I of BLAST development is scheduled for January - April of this year and is focused on creating lightweight models of common LN node implementations. In this context I am defining `model` as a software application that emulates how a real Lightning node operates, but it has been optimized for a simulation environment. By optimizing these models I hope to be able to run test networks with many nodes and channels. Because LND is written in `Go` and `Go` has good profiling tools built into it, and because LND is a widely used implementation, I decided to start there. I am about 3 months into the proposed 4 month phase and have made some great progress researching how LND uses memory.

In order to get to the end goal (a full simulation tool), I first have to create lightweight models of the nodes. And in order to create lighweight models of the nodes, I first have to understand how the real nodes use resources. Therefore, the first step is to profile the node software and gain an understanding of what parts of it use the most resources.

```mermaid
flowchart LR
    sim[Large Scale LN Simulator] --> models[Lightweight Models] --> understand[Understand Real Nodes] --> profile[Profile Real Nodes]
```

The `pprof` tool in `Go` is a powerful performance profiling tool that allows developers to analyze the runtime behavior of their `Go` programs. It provides insights into CPU usage, memory allocation, blocking calls, and various other metrics.

Here are some key features of `pprof` that made it useful for this work:

**Instrumentation:** `pprof` relies on the `Go` runtime's ability to dynamically instrument running code. This means it can gather runtime statistics without requiring the program to be modified or recompiled.

**Profiling Types:**
- CPU Profiling: This type of profiling captures how much time the program spends executing different functions. It samples the program's stack at regular intervals to determine where it's spending its time.

- Memory Profiling: Memory profiling helps identify memory allocations and potential leaks by analyzing how much memory is being used and which parts of the code are responsible for the allocations.

- Block Profiling: Block profiling helps in identifying blocking calls. It shows which goroutines are blocked and what they are waiting for.

**Profiling Endpoints:** `pprof` provides HTTP endpoints that expose profiling data in a format that can be analyzed by various tools. These endpoints can be enabled by importing the net/http/pprof package and adding a few lines of code to your program.

**Using the go tool pprof command:** Once the profiling endpoints are enabled, you can use the `go tool pprof` command to analyze the collected data. You can generate reports, visualize profiles, and even interactively explore the profile data.

To start, I developed a Docker container equipped with essential memory profiling and resource monitoring tools, alongside the LND and Bitcoin Core software. Additionally, the container includes a block explorer and Python for performance analysis and graphical representation of both lnd and Bitcoin Core.

Using `pprof`, I can see what functions allocate the most memory and which ones could potentially be altered to use less memory in a simulation environment. The general approach of BLAST is to find parts of LN node implementations that are not important in a testing use case and tweak those parts to use less resources so that large test networks can be created. Once I had all the tools in place and LND setup correctly I executed the following steps:
 1. Run an LND instance in the Docker container.
 2. Use a python script to query the `pprof` tool at regular intervals as LND runs.
 3. Save memory stats to a csv file.
 4. Use another python script to generate visuals of the csv data using `matplotlib` so that I can easily analyze the LND memory usage.

 I followed this process for several different scenarios, including:
 - An idle LND with 0 channels
 - An idle LND with 5 channels
 - An idle LND with 9 channels
 - An idle LND with 30 channels
 - LND while it opens 30 channels
 - LND while processing transactions
 - LND while processing lots of transactions

After doing this analysis on one LND instance, I researched `goroutines` to see if they could help me run multiple LND instances more efficiently. One benefit of LND being written in `go` is that `goroutines` are an efficient way to handle concurrency. BLAST will leverage `goroutines` to start multiple LND nodes instead of running multiple nodes as separate processes. This will help save on host resources and give BLAST more control over each node.

In general, starting a new thread within a process is more memory-efficient than starting a new process. A few reasons why:

1. Creating a new process involves duplicating the entire address space of the parent process, including code, data, and stack segments. This incurs a significant memory overhead because each process has its own memory space. On the other hand, threads within a process share the same address space, including code and data segments, which results in less memory overhead.

2. Context switching between threads within the same process is generally faster than between processes because the kernel doesn't need to perform memory mapping for different address spaces. Context switching between threads usually involves switching the CPU registers and stack pointers, which is faster compared to the overhead of switching between processes.

3. Threads within the same process share resources such as file descriptors, memory allocations, and other operating system resources, which can lead to better resource utilization compared to separate processes.

`goroutines`, a concurrency primitive in the `Go` programming language, are often more efficient than threads in traditional operating systems for several reasons:

1. `goroutines` are lightweight compared to threads. They have smaller stack sizes by default (a few kilobytes), which reduces the memory overhead per concurrent task. This allows `Go` programs to efficiently handle large numbers of concurrent tasks without consuming excessive memory.

2. `goroutines` are designed to have fast context switching. Switching between goroutines involves changing the stack and program counter, which is faster than the context switching overhead of traditional threads in many cases. This efficiency is achieved through cooperative scheduling managed by the `Go` runtime, which avoids the overhead of preemptive context switches used in traditional threading models.

3. `Go`'s garbage collector is optimized for concurrent execution and works seamlessly with `goroutines`. It can efficiently reclaim memory allocated by `goroutines` without causing significant pauses or overhead.

After setting up a way to run multiple LND instances in `goroutines`, I repeated the profiling process on 100 LND nodes, each running in its own `goroutine`.

While doing this analysis, I noticed that most of the memory requirements of LND are used when starting up the node. I took a closer look at this using the `-alloc_space` flag of `pprof`.

When using the `pprof` tool you can use the `-inuse_space` flag or the `-alloc_space` flag:

- `-inuse_space` shows the total number of bytes currently allocated (live objects, not garbage collected or released back to the OS)
- `-alloc_space` shows the total number of bytes allocated since the program began (including garbage-collected bytes)

This showed that creating encryption keys during the node startup requires a lot of memory. I started experimenting with ways to reduce this memory requirement.

A full write up about the LND profiling work can be found [here](https://github.com/bjohnson5/blast/blob/main/research/phase1/lnd/README.md).

## Met with `warnet` and `sim-ln` Developers to Discuss Collaboration
Early on in this quarter, I had the opportunity to meet with developers of two new Lightning Network simulation projects, [warnet](https://github.com/bitcoin-dev-project/warnet) and [sim-ln](https://github.com/bitcoin-dev-project/sim-ln) to explore potential similarities and collaboration opportunities between our respective projects. During the meeting, I engaged in discussions on the similarities between our projects and identified areas where collaboration could be possible. Since the meeting, I have maintained communication with contributors of `sim-ln`, actively participating in the weekly Discord update thread, engaging in discussions on GitHub regarding code changes, and ultimately making a meaningful contribution to `sim-ln`. This ongoing collaboration reflects our shared commitment to advancing Lightning Network Modeling and Simulation projects and leveraging collective expertise for the betterment of our projects and the broader Lightning community.

The `sim-ln` project stood out to me as a unique and useful tool. If you have a network of Lightning nodes, you can run `sim-ln` on that network and it will generate payments between nodes and log the results of those payments. It can operate in a random mode that generates general network traffic or in a defined mode where the user can specify what payments are made and how often. BLAST will eventually allow a user to start up a large test Lightning Network and it will need a payment generator to create transactions on the network. I believe that BLAST and `sim-ln` will work perfectly together, so I decided to start getting involved in its development.

### SimLn Contribution
After discussing what `sim-ln` does with its author and diving into the code, it became clear that `sim-ln` will integrate very well with BLAST. The `sim-ln` project is a payment generator for the Lightning Network. From the `sim-ln` README:

<blockquote>
sim-ln is a simulation tool that can be used to generate realistic payment activity on any lightning network topology. It is intentionally environment-agnostic so that it can be used across many environments - from integration tests to public signets.
</blockquote>

I quickly realized that one thing that `sim-ln` was lacking was flexibility in the defined payment activity mode. BLAST will need to let users have complete control over when and how often payments are made. I opened Issue [#168](https://github.com/bitcoin-dev-project/sim-ln/issues/168) to address this and began implementing it. From the issue description:

<blockquote>
Describe desired feature: <br><br>
Currently when you are defining specific activity you can only define an interval and amount. It would be helpful to be able to pick a start time and duration so that some nodes are sending transactions at different times throughout the simulation.
<br><br>
Use case for feature:<br><br>
For example:<br>
This activity definition would tell Alice to send 2000 msats every second, starting 10 seconds after the simulation starts and ending after 30 seconds. In other words, Alice would send Bob 30 transactions between 10 and 40 seconds in the simulation.
</blockquote>

```
"activity": [
    {
      "source": "Alice",
      "destination": "Bob",
      "start": 10,
      "duration": 30
      "interval_secs": 1,
      "amount_msat": 2000
    }
  ]
```

After completing the implementation of this feature and doing the proper testing. I opened PR [#173](https://github.com/bitcoin-dev-project/sim-ln/pull/173) and worked with the other contributors to review and clean up the feature. Multiple contributors reviewed the PR and provided excellent feedback. I updated the PR with the suggested changes and this PR has now been merged into `sim-ln`.

## Developing BLAST
Now that the initial LND model has started to take shape, developing it in conjuction with the core BLAST libraries is important. An overall design for BLAST can be seen [here](https://github.com/bjohnson5/blast/blob/main/design/blast_design.md). The idea is to create a library (`blast_core`) that can be used to start up networks and automate test scenarios. The models (`blast_models`) can be developed independently and be plugged into BLAST by implementing the BLAST RPC protocol (`blast_proto`). By implementing that protocol the models can be controlled with the `blast_model_interface` so that automated scenarios can be run. The two user facing applications will be `blast_cli` and `blast_web`. Real nodes can also be manually connected to the network and controlled directly by the user during the scenarios, but in order for BLAST to perform automated operations the node must implement the BLAST RPC protocol.

The BLAST repository is now set up for continued development of:
- `blast_cli` - The CLI for running BLAST
- `blast_core` - The core simulation library
- `blast_model_interface` - The interface between the node models and core library
- `blast_models` - The node models
- `blast_proto` - The BLAST RPC protocol definition
- `blast_web` - The web interface for running BLAST

Some basic test code has been written to validate this design, but these components will be continually improving through the duration of the project.

## Up Next
- Complete the LND model.
- Write the initial parts of `blast_core` that will enable the starting of lots of LND nodes running behind the LND model.
- Perform the profiling process on the completed LND model and compare the results to the original benchmarks.
- Phase II: Research and develop automation techniques and reproducible network states.
- Phase III: Integrate the results of Phase I and Phase II into the BLAST tool.

## What was the OpenSats funding used for?
The grant money was used as part of my income in order to pay for living expenses.
