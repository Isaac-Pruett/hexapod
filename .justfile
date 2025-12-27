alias r := run
alias b := build
alias c := clean

# default target, lists all targets available
list:
    @just --list

run:
    @nix run

build:
    @nix build

clean:
    @rm -rf result

nuke: clean
    @nix-collect-garbage
