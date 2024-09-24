# LND Deadlock
A potential deadlock was discovered using BLAST. By creating a simulation with several nodes and channels and using SimLn to generate network traffic a mutex deadlock was found that caused an LND node to freeze up and quit processing peer messages or RPC calls. A few important points to note:

- BLAST was using LND nodes with the `graphCache` turned off to save memory. This is not the default configuration so this bug went unnoticed. However, if other LND users are trying to save memory they may run without the `graphCache` and are then vulnerable to this issue.
- SimLN was vital in finding this problem because it generated enough network traffic that this problem was occuring regularly. Manually sending a few transactions from one node to another would not have triggered the deadlock. It was found because the node was in the process of finding a route for its next payment and also managing incoming messages from its peers about previous transactions. This multi-tasking was caused by the automation of BLAST and SimLN and is ultimately responsible for uncovering the bug.
- The repeadablity that BLAST allows for is what enabled the troubleshooting of this bug and understanding what the underlying problem was. We started noticing the nodes freezing up during a BLAST simulation and so we saved the simulation and began running it again. Because the saved simulation executes the same way each time we repeatedly were seeing the freezing issue. Debugging was possible because we could start and stop the simulation over and over again and observe what was happening in the LND code.

This is an excellent example of the use case for testing tools like BLAST and SimLN. Research like this will uncover edge case conditions like this particular deadlock that can then be fixed and make lightning node software more reliable and secure.

# Technical Analysis
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

This issue has been created in the LND repo:

https://github.com/lightningnetwork/lnd/discussions/9060

https://github.com/lightningnetwork/lnd/issues/9133
