#!/usr/bin/env sh

# put this as ics26 address in config

jq -r '.returns."0".value' ../solidity-ibc-eureka/broadcast/E2ETestDeploy.s.sol/80087/run-latest.json | sed -e 's#\\\"#"#g' | jq ".$1"
