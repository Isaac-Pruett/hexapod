alias r := run
alias b := build
alias d := develop

list:
    just --list

# builds and uploads to the connected RP1-RP2 drive
run:
    cargo r --release

# builds the release binary
build:
    cargo b --release

# builds and uploads the dev binary
develop:
    cargo r
