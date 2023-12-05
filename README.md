# eve-graph
An application for building a graph database representation of universe data in EVE.

## Current State
The application is still currently in a rapid prototyping phase. As such, there are no tests, and significant rewrites
should be expected.

Currently, the application will:
* Call the public EVE Swagger Interface (ESI) for a list of systems in the universe.
* Parse the list of systems to a vector of integer system ids.
* Concurrently for each system id, fetch the details of the system and write them to a local neo4j database.

Known issues:
* Current structures that represent EVE concepts do not yet model all the variations in the ESI system data (ie some developer only systems are missing stargates entirely).
* High concurrency becomes an issue when the ESI API returns an error. While each response contains an error allowance and backoff timer, this application does not currently backoff and will cascade into being rate limited.

Next steps:
* Build a dead letter queue of system ids for system detail queries that fail.
* Implement a check which will backoff ESI queries when failures are observed or rate limiting begins.
* Iterate through specific system detail query failures and adjust the corresponding structures.

## How to run
You will need a locally installed neo4j service. User and pass should be `neo4j` and `neo4jneo4j` respectively, or change them yourself in `main.rs` as they are temporarily hard coded for convenience.

Build the crate, and run the tokio main block in `main.rs`. You will likely want to manually abort with `ctrl + c` if you observe failures being logged to stdout.