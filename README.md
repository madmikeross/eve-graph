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

### Finding a safe route
In game, you can route via only high security systems, while this may seem safe, you can always be attacked in game.
In addition to the shortest route, you can have eve_graph suggest a safe route based on how many kills have recently
occurred in the system. Currently, this feature requires a bit of manual effort, but will become an endpoint soon.

First, you need to already have built the database as above (systems, stargates, and wormholes). Then you need to call
the endpoint to assign a risk to each jump:
```bash
curl -X POST 127.0.0.1:8008/systems/risk
```

Next, you need to build the `jump-risk` graph in neo4j. Refer to the `build_jump_risk_graph` function in the database
module for the query you should run. Last, you need to run a query similar to the `find_shortest_route` function in the
database module with a couple modifications in order to find the safest path. Simply substitute `jump-risk` for
`system-map` and `risk` for `cost` (and put in your source and destination system names) and you should have a "safe" route
which is also likely shorter than the high sec route.

### Running with Docker
A fully functioning docker build for the app is not yet complete. Neo4j will come up, but we still need to install the
data science plugin. The app will start, but requests to ESI from within the container are completing too quickly, and
better logging needs to be added to debug the issue further.