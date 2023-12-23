# eve-graph
An application for finding optimal routes between systems in EVE Online.

## How to use
### Running with Docker
Make sure you have Docker engine installed, then run `docker compose up`. This should build a container for Neo4j,
install the graph-data-science plugin, and also build a container for the eve-graph app.

### Collecting data
You need to exercise the system refresh, stargate refresh, and wormhole refresh endpoints to hydrate the database
with data on first run. Also, every time you restart the database, the in memory "graph" of data that the gds plugin
uses will need to be rebuilt, calling to refresh wormholes also refreshes this "graph" (and you should call to refresh
wormholes regularly).

### Finding the shortest route
If you want to find the shortest route between two systems, say Cleyd and Jita, simply issue a get request to
`127.0.0.1:8008/routes/Amarr/to/Jita` (can be done in a browser, with curl, or via Postman).

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
module for the query you should run (run these queries in the Neo4j browser http://localhost:7474/browser/). Last, you
need to run a query similar to the `find_shortest_route` function in the database module with a couple modifications in
order to find the safest path. Simply substitute `jump-risk` for `system-map` and `risk` for `cost` (and put in your
source and destination system names) and you should have a "safe" route which is also likely shorter than the high sec
route.