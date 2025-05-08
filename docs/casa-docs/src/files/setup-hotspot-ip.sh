#!/bin/bash
sleep 60s
echo "Attempting to add address"
date &>> /home/casa/setup-hotspot.logs
ip addr add 10.0.0.1/24 dev wlP1p1s0 &>> /home/casa/setup-hotspot.logs
