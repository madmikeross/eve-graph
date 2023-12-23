# eve-graph
An application for finding optimal routes between systems in EVE Online.

## How to use
### Running with Docker
Make sure you have Docker engine installed, then run `docker compose up`. This should build a container for Neo4j,
install the graph-data-science plugin, and also build a container for the eve-graph app.

On start, eve-graph will attempt to synchronize systems and stargates with ESI before accepting requests. If routing
isn't working properly, inspect the logs for the api container.

### Collecting data
When you first start eve-graph, you will need to exercise the wormhole refresh endpoint to hydrate the database 
ephemeral J-space connections. Also, every time you restart the database, the in memory "graph" of data that the gds 
plugin uses will need to be rebuilt, calling to refresh wormholes also refreshes this "graph" (and you should call to 
refresh wormholes regularly).

### Finding the shortest route
If you want to find the shortest route between two systems, say Jita and Amarr, simply issue a get request to
`localhost:8008/shortest-route/Jita/to/Amarr` (can be done in a browser, with curl, or via Postman).

### Finding a safe route
First you need to update the jump risks of each system by making a POST request to `localhost:8008/systems/risk`. This
will fetch kills and jumps for all systems in the last hour, and use those values to assign a risk to jumping into each
system. If you want to find a safe route between two systems, say Jita and Amarr, issue a get request to
`127.0.0.1:8008/safest-route/Amarr/to/Jita`