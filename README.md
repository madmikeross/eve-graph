# eve-graph

A route finding application for navigating in EVE Online.

## How to use

### Running with Docker

Make sure you have Docker engine installed, then run `docker compose up -d`. This should build a container for Neo4j,
install the graph-data-science plugin, and also build a container for the eve-graph app.

On start, eve-graph will attempt to synchronize systems and stargates with [ESI](https://esi.evetech.net/ui/) before
accepting requests. If routing isn't working properly, inspect the logs for the api container
`docker logs eve-graph-api-1`.

### Finding the shortest route

If you want to find the shortest route between two systems, say Jita and Amarr, simply issue a get request to
`localhost:8008/shortest-route/Jita/to/Amarr` (can be done in a browser, with curl, or via Postman).

### Finding a safe route

If you want to find a safe route between two systems, say Jita and Amarr, issue a get request to
`localhost:8008/safest-route/Amarr/to/Jita`.

## Ephemeral data

If it has been a while since you started the app, you should refresh wormhole connections by making a POST request to
`localhost:8008/wormholes/refresh`. These wormhole connections come from [EVE Scout](https://www.eve-scout.com/#/) and
are used in both the short and safe route path finding solutions.

If you are using the safe routing feature, it is also good to regularly recompute (about every 30 minutes) the risk of
jumping into each system by making a POST request to `localhost:8008/systems/risk`.