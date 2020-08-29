# CraftQL ![ci](https://github.com/yamafaktory/craftql/workflows/ci/badge.svg)

> A CLI tool to manipulate GraphQL schemas and to output a graph data structure as a graphviz .dot format

## Installation

```sh
cargo install craftql
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
    0 [ label = "DateTime (Scalar)" ]
    1 [ label = "Character (Interface extension)\l\l[Boolean, Character]" ]
    2 [ label = "Human (Object)\l\l[Character, Episode, Float, FriendsConnection, ID, Int, LengthUnit, Starship, String]" ]
    3 [ label = "Droid (Object)\l\l[Character, Episode, FriendsConnection, ID, Int, String]" ]
    4 [ label = "FriendsConnection (Object)\l\l[Character, FriendsEdge, Int, PageInfo]" ]
    5 [ label = "FriendsEdge (Object)\l\l[Character, ID]" ]
    6 [ label = "PageInfo (Object)\l\l[Boolean, ID, test]" ]
    7 [ label = "Review (Object)\l\l[DateTime, Episode, Int, String, test]" ]
    8 [ label = "Orphan (Object)\l\l[ID]" ]
    9 [ label = "ColorInput (InputObject extension)\l\l[ColorInput, Int]" ]
    10 [ label = "Episode (Enum extension)\l\l[Episode]" ]
    11 [ label = "deprecated (Directive)\l\l[String]" ]
    12 [ label = "DateTime (Scalar extension)\l\l[DateTime, test]" ]
    13 [ label = "SearchResult (Union extension)\l\l[Ewok, SearchResult]" ]
    14 [ label = "test (Directive)\l\l[Letter]" ]
    15 [ label = "Episode (Enum)\l\l[deprecated, test]" ]
    16 [ label = "LengthUnit (Enum)" ]
    17 [ label = "Starship (Object extension)\l\l[Boolean, Starship]" ]
    18 [ label = "Query (Object)\l\l[Character, Droid, Episode, Human, ID, Review, SearchResult, Starship, String]" ]
    19 [ label = "Mutation (Object)\l\l[Episode, Review, ReviewInput]" ]
    20 [ label = "Subscription (Object)\l\l[Episode, Review]" ]
    21 [ label = "schema (Schema)\l\l[Mutation, Query, Subscription]" ]
    22 [ label = "ReviewInput (InputObject)\l\l[ColorInput, Int, ReviewInput, String]" ]
    23 [ label = "ColorInput (InputObject)\l\l[ColorInput, deprecated, Int, test]" ]
    24 [ label = "Letter (Enum)" ]
    25 [ label = "Starship (Object)\l\l[deprecated, Float, ID, LengthUnit, String]" ]
    26 [ label = "Character (Interface)\l\l[Bool, Character, deprecated, Episode, FriendsConnection, ID, Int, String, test]" ]
    27 [ label = "SearchResult (Union)\l\l[Droid, Human, Starship, test]" ]
    13 -> 27 [ ]
    14 -> 6 [ ]
    26 -> 4 [ ]
    5 -> 4 [ ]
    6 -> 4 [ ]
    11 -> 25 [ ]
    16 -> 25 [ ]
    26 -> 26 [ ]
    11 -> 26 [ ]
    15 -> 26 [ ]
    4 -> 26 [ ]
    14 -> 26 [ ]
    26 -> 3 [ ]
    15 -> 3 [ ]
    4 -> 3 [ ]
    17 -> 25 [ ]
    15 -> 20 [ ]
    7 -> 20 [ ]
    19 -> 21 [ ]
    18 -> 21 [ ]
    20 -> 21 [ ]
    9 -> 23 [ ]
    26 -> 18 [ ]
    3 -> 18 [ ]
    15 -> 18 [ ]
    2 -> 18 [ ]
    7 -> 18 [ ]
    27 -> 18 [ ]
    25 -> 18 [ ]
    26 -> 2 [ ]
    15 -> 2 [ ]
    4 -> 2 [ ]
    16 -> 2 [ ]
    25 -> 2 [ ]
    1 -> 26 [ ]
    15 -> 19 [ ]
    7 -> 19 [ ]
    22 -> 19 [ ]
    23 -> 22 [ ]
    22 -> 22 [ ]
    23 -> 23 [ ]
    11 -> 23 [ ]
    14 -> 23 [ ]
    24 -> 14 [ ]
    12 -> 0 [ ]
    12 -> 14 [ ]
    11 -> 15 [ ]
    14 -> 15 [ ]
    10 -> 15 [ ]
    0 -> 7 [ ]
    15 -> 7 [ ]
    14 -> 7 [ ]
    26 -> 5 [ ]
    3 -> 27 [ ]
    2 -> 27 [ ]
    25 -> 27 [ ]
    14 -> 27 [ ]
}
```

![graph](graph.svg)

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

### Find and display multiple nodes

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
- Add GitHub CI
- More to come!
