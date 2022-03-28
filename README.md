# Senior Software Engineer Take-Home Programming Assignment for Rust
https://docs.google.com/document/d/1at48ur7pCN8tOG6wLh3B5pOroIzla3IAI4cH2tZSjek/edit#heading=h.rlnlxzzcsgwh

Writer : @Kyle Wilson 2022/03/26 


## How to Install & Run backend?

Standard rust cargo multi-binary setup:
```
$ cargo build
$ cargo run --bin flights
```

### Command line help

Also, limited options are available at the command line.  Run `--help`
to view available options:

```
$ cargo run --bin flights -- --help
```

## Server API

Connect to HTTP endpoint using any web client.

### API: PUT (store key and value)

Meta-request: PUT http://$HOSTNAME:$PORT/api/$DB/$KEY

Append the key to the URI path, and provide HTTP body as value.  In the
following example, "flights" is the key, "[['IND', 'EWR'], ['SFO', 'ATL'], ['GSO', 'IND'], ['ATL', 'GSO']]" is the value,
and "/api/db/flights" is the base URI:
```
curl --data-binary [['IND', 'EWR'], ['SFO', 'ATL'], ['GSO', 'IND'], ['ATL', 'GSO']] -X PUT http://localhost:8080/api/db/flights
```

Returns JSON indicating success:
```
{
    "result": [
        [
            "SFO",
            "EWR"
        ]
    ]
}

```
