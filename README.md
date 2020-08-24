# CraftQL

> A CLI tool to manipulate GraphQL schemas and to output a graph data structure as a graphviz .dot format

## Installation

```sh
cargo install jql
```

## Usage

```sh
USAGE:
    craftql [FLAGS] [OPTIONS] <path>

ARGS:
    <path>    Path to get files from

FLAGS:
    -h, --help       Prints help information
    -o, --orphans    Finds and display orphan(s) node(s)
    -V, --version    Prints version information

OPTIONS:
    -n, --node <node>         Finds and display one node
    -N, --nodes <nodes>...    Finds and display multiple nodes
```

### Output a graphviz .dot format

```sh
craftql tests/fixtures

digraph {
    0 [ label = "ColorInputExt (InputObject extension) [\"Int\", \"ColorInput\"]" ]
    1 [ label = "Starship (Object) [\"ID\", \"String\", \"deprecated\", \"String\", \"LengthUnit\", \"Float\", \"Float\"]" ]
    2 [ label = "StarshipExt (Object extension) [\"Boolean\", \"Starship\"]" ]
    3 [ label = "LengthUnit (Enum) []" ]
    4 [ label = "SearchResult (Union) [\"Human\", \"Droid\", \"Starship\", \"test\"]" ]
    5 [ label = "SearchResultExt (Union extension) [\"Ewok\", \"SearchResult\"]" ]
    6 [ label = "Episode (Enum) [\"test\", \"deprecated\"]" ]
    7 [ label = "ReviewInput (InputObject) [\"Int\", \"String\", \"ColorInput\"]" ]
    8 [ label = "ColorInput (InputObject) [\"Int\", \"Int\", \"Int\", \"deprecated\", \"Int\", \"test\"]" ]
    9 [ label = "CharacterExt (Interface extension) [\"Boolean\", \"Character\"]" ]
    10 [ label = "Character (Interface) [\"ID\", \"String\", \"Character\", \"Int\", \"ID\", \"FriendsConnection\", \"Episode\", \"deprecated\", \"Bool\", \"test\"]" ]
    11 [ label = "Query (Object) [\"Episode\", \"Character\", \"Episode\", \"Review\", \"String\", \"SearchResult\", \"ID\", \"Character\", \"ID\", \"Droid\", \"ID\", \"Human\", \"ID\", \"Starship\"]" ]
    12 [ label = "Mutation (Object) [\"Episode\", \"ReviewInput\", \"Review\"]" ]
    13 [ label = "Subscription (Object) [\"Episode\", \"Review\"]" ]
    14 [ label = "EpisodeExt (Enum extension) [\"Episode\"]" ]
    15 [ label = "deprecated (Directive) [\"String\"]" ]
    16 [ label = "Letter (Enum) []" ]
    17 [ label = "DateTime (Scalar) []" ]
    18 [ label = "test (Directive) [\"Letter\"]" ]
    19 [ label = "schema (Schema) [\"Query\", \"Mutation\", \"Subscription\"]" ]
    20 [ label = "DateTimeExt (Scalar extension) [\"test\", \"DateTime\"]" ]
    21 [ label = "Orphan (Object) [\"ID\"]" ]
    22 [ label = "Human (Object) [\"ID\", \"String\", \"String\", \"LengthUnit\", \"Float\", \"Float\", \"Character\", \"Int\", \"ID\", \"FriendsConnection\", \"Episode\", \"Starship\", \"Character\"]" ]
    23 [ label = "Droid (Object) [\"ID\", \"String\", \"Character\", \"Int\", \"ID\", \"FriendsConnection\", \"Episode\", \"String\", \"Character\"]" ]
    24 [ label = "FriendsConnection (Object) [\"Int\", \"FriendsEdge\", \"Character\", \"PageInfo\"]" ]
    25 [ label = "FriendsEdge (Object) [\"ID\", \"Character\"]" ]
    26 [ label = "PageInfo (Object) [\"ID\", \"ID\", \"Boolean\", \"test\"]" ]
    27 [ label = "Review (Object) [\"Episode\", \"test\", \"Int\", \"String\", \"DateTime\"]" ]
    8 -> 7 [ ]
    6 -> 27 [ ]
    18 -> 27 [ ]
    17 -> 27 [ ]
    6 -> 13 [ ]
    27 -> 13 [ ]
    16 -> 18 [ ]
    18 -> 26 [ ]
    18 -> 6 [ ]
    15 -> 6 [ ]
    5 -> 4 [ ]
    2 -> 1 [ ]
    14 -> 6 [ ]
    10 -> 10 [ ]
    24 -> 10 [ ]
    6 -> 10 [ ]
    15 -> 10 [ ]
    18 -> 10 [ ]
    22 -> 4 [ ]
    23 -> 4 [ ]
    1 -> 4 [ ]
    18 -> 4 [ ]
    11 -> 19 [ ]
    12 -> 19 [ ]
    13 -> 19 [ ]
    15 -> 8 [ ]
    18 -> 8 [ ]
    20 -> 18 [ ]
    20 -> 17 [ ]
    10 -> 23 [ ]
    24 -> 23 [ ]
    6 -> 23 [ ]
    25 -> 24 [ ]
    10 -> 24 [ ]
    26 -> 24 [ ]
    10 -> 25 [ ]
    0 -> 8 [ ]
    6 -> 11 [ ]
    10 -> 11 [ ]
    27 -> 11 [ ]
    4 -> 11 [ ]
    23 -> 11 [ ]
    22 -> 11 [ ]
    1 -> 11 [ ]
    15 -> 1 [ ]
    3 -> 1 [ ]
    6 -> 12 [ ]
    7 -> 12 [ ]
    27 -> 12 [ ]
    9 -> 10 [ ]
    3 -> 22 [ ]
    10 -> 22 [ ]
    24 -> 22 [ ]
    6 -> 22 [ ]
    1 -> 22 [ ]
}
```

### Find and display one node

```sh
craftql tests/fixtures --node Character

# tests/fixtures/Types/Interfaces/Character.graphql
interface Character @test {
  id: ID!
  name: String!
  friends: [Character]
  friendsConnection(first: Int, after: ID): FriendsConnection!
  appearsIn: [Episode]!
  cute: Bool! @deprecated
}
```

### Find and display one node

```sh
craftql tests/fixtures --nodes Character Episode

# tests/fixtures/Types/Interfaces/Character.graphql
interface Character @test {
  id: ID!
  name: String!
  friends: [Character]
  friendsConnection(first: Int, after: ID): FriendsConnection!
  appearsIn: [Episode]!
  cute: Bool! @deprecated
}


# tests/fixtures/Types/Enums/Episode.gql
enum Episode @test(letter: B) {
  NEWHOPE @deprecated
  EMPIRE
  JEDI
}
```

### Find and display orphan(s) node(s)

```sh
craftql tests/fixtures --orphans

# tests/fixtures/Types/Types/orphan.gql
type Orphan {
  id: ID!
}
```

## TODO

- Add flag to list dependencies for a given node
- Add tests
- Add GitHub CI
- More to come!