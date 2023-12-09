# eve-graph
An application for building a graph database representation of universe data in EVE.

## Current State
The application is still currently in a rapid prototyping phase. As such, there are no tests, and significant rewrites
should be expected.

Currently, the application will:
* Call the public EVE Swagger Interface (ESI) for a list of systems in the universe.
* Save the systems, and their stargates, as nodes in the database.
* Pull public signatures from eve scout, and save wormhole connections between Thera and Turnur

Next steps:
* Add a function that programmatically executes the shortest path query between systems
* Support route finding filters like security and avoidance lists

## How to run
You will need a locally installed neo4j service. User and pass should be `neo4j` and `neo4jneo4j` respectively, or
change them yourself in `main.rs` as they are temporarily hard coded for convenience. You need to install the data
science library. Follow these [installation instructions](https://neo4j.com/docs/graph-data-science/current/installation/neo4j-server/).

Build the crate, and run the tokio main block in `main.rs`. After main finishes running, you should have a connected
graph of systems in the database.

You can manually query for shortest path (including Thera and Turnur traversals) using the following queries in the
neo4j browser:

Build the graph:
```genericsql
CALL gds.graph.project(
    'system-map',
    'System',
    'JUMP',
    {
        relationshipProperties: 'cost'
    }
)
```

Get a shortest path from Cadelanne (some LS exit hole) to Jita:
```genericsql
MATCH (source:System {name: 'Jita'}), (target:System {name: 'Cadelanne'})
CALL gds.shortestPath.dijkstra.stream('system-map', {
    sourceNode: source,
    targetNode: target,
    relationshipWeightProperty: 'cost'
})
YIELD index, sourceNode, targetNode, totalCost, nodeIds, costs, path
RETURN
    index,
    gds.util.asNode(sourceNode).name AS sourceNodeName,
    gds.util.asNode(targetNode).name AS targetNodeName,
    totalCost,
    [nodeId IN nodeIds | gds.util.asNode(nodeId).name] AS nodeNames,
    costs,
    nodes(path) as path
ORDER BY index
```

Dance because you don't have to big brain Thera stuff anymore, just fly the route `["Jita", "New Caldari", "Alikara", "Kaimon", "Kausaaja", "Auviken", "Ohvosamon", "Thera", "PVH8-0", "JH-M2W", "M2-CF1", "97X-CH", "Y9G-KS", "Conomette", "Aimoguier", "Cadelanne"]	`