#!/bin/bash

# Syntax:  [CPU_num] [Radiation dose]
# example ./CPUbenchmark 1 10krad
# Benchmarking AMD CPU



./CPUbenchmark1GHz.sh
./CPUbenchmark2GHz.sh
./CPUbenchmark3GHz.sh
#./stress.sh
core-to-core-latency -b 1 --csv > latencyOne
core-to-core-latency -b 2 --csv > latencyTwo
core-to-core-latency -b 3 --csv > latencyThree


stress-ng --intmath 20 --fp 20 --prime 20 --bubblesort 20 --crypt 20 --jpeg 20 --ipsec-mb 20 --cache 20 --mcontend 20 --cylic 20 -t 60 -mlock-ops 20 --log-file op3 --metrics
