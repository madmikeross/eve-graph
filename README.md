# eve-graph
An application for discovering routes between Systems in EVE.

## Current State
The application is still currently in a rapid prototyping phase. As such, there are few tests, and significant rewrites
should be expected.

Currently, the application will:
* Build a graph database of systems and their connections (via stargates and wormholes)
* Provide shortest-path route finding between two systems

Next steps:
* Move database initialization steps to route handlers
* Support route finding filters like security and avoidance lists

## How to run
You will need a locally installed neo4j service. User and pass should be `neo4j` and `neo4jneo4j` respectively, or
change them yourself in `main.rs` as they are temporarily hard coded for convenience. You need to install the data
science library. Follow these [installation instructions](https://neo4j.com/docs/graph-data-science/current/installation/neo4j-server/).

Build the crate, and run the commented out lines of the tokio main block in `main.rs`. After main finishes running, you
should have a connected graph of systems in the database (this is a temporary transition step from prototype to api).
Now you may comment out the database building and run the warp configuration, make a POST to `127.0.0.1:8008/wormholes/refresh` to
fetch the current wormhole signatures from eve scout and build the system graph. Now you can issue a request for a route
in your browser with a GET request to `127.0.0.1:8008/routes/Cleyd/to/Jita`.