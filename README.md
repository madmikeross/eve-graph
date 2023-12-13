# eve-graph
An application for finding optimal routes between systems in EVE Online.

## How to use
### Pre-requisites
You will need a locally installed neo4j service. User and pass should be `neo4j` and `neo4jneo4j` respectively, or
change them yourself in `main.rs` as they are temporarily hard coded for convenience. After installing neo4j, you need
to install the data science library. Follow these [installation instructions](https://neo4j.com/docs/graph-data-science/current/installation/neo4j-server/).

In order to build the application and run it, you will need to [install Rust](https://www.rust-lang.org/tools/install).

### Building the database
Run the crate `cargo run` to start a local web server at `127.0.0.1:8008`. Next you need to pull down the public system
stargate and wormhole data with a series of requests (you can use Postman to issue POST requests if you prefer a gui).
```bash
curl -X POST 127.0.0.1:8008/systems/refresh
curl -X POST 127.0.0.1:8008/stargates/refresh
curl -X POST 127.0.0.1:8008/wormholes/refresh
```
These requests should each take a few seconds to complete, but if you are waiting minutes, something has gone wrong.

### Finding the shortest route
If you want to find the shortest route between two systems, say Cleyd and Jita, simply issue a get request to
`127.0.0.1:8008/routes/Cleyd/to/Jita` (can be done in a browser, with curl, or via Postman).