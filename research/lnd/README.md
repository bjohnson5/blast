# LND Profiling

## TEST CASE 1
Idle LND instance with 0 channels
in use space

## TEST CASE 2
Idle LND instance with 5 channels
in use space

## TEST CASE 3
Idle LND instance with 9 channels
in use space

## TEST CASE 4
Idle LND instance with 30 channels
in use space

## TEST CASE 5
LND instance during the process of opening 30 channels
in use space

## TEST CASE 6
LND instance during lots of transactions
in use space

## TEST CASE 7
LND instance during more transactions
in use space

## TEST CASE 8
100 LND instances separated by 10 seconds
in use space

## TEST CASE 9
100 LND instances separated by 10 seconds
in use space
no caches

## TEST CASE 10
100 LND instances separated by 10 seconds
in use space
no caches
neutrino

## TEST CASE 11
100 LND instances separated by 10 seconds
total allocs
no caches
neutrino
during create wallet

## TEST CASE 12
100 LND instances separated by 10 seconds
total allocs
no caches
neutrino
during open wallet

## TEST CASE 13
100 LND instances separated by 10 seconds
total allocs
no caches
neutrino
using shared wallet