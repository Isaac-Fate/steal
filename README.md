# Steal

A CLI tool for multipart downloading. 

## Motivation

Downloading large datasets and other resources in a server is slow using `wget` since it is a single-threaded downloader.
So, I want to write a Rust CLI tool that can download multiple files in parallel.

## Installation

### Installing from Source

- Clone my GitHub repository https://github.com/Isaac-Fate/steal.git
- Navigate to `steal` directory
- Run `cargo install --path .` to install it locally

```sh
git clone https://github.com/Isaac-Fate/steal.git
cd steal
cargo install --path .
```

## Usage

Type `steal help` to see all available commands.
