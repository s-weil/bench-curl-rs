# bench-curl-rs

Statistics-driven benchmarking of http-webapps, in Rust.

<img src="./examples/box_plot.jpg" width="700" height="600" />


### TODO:
* dump also a table of stats into the results folder
* http examples for testing
* provide param for measuring in milli/micro/nano
* cli
* plotly
* tokio support (tbd)
* rayon support
* -> support jupityer notebooks! via python api
* parallel via rayon?
* input randomizer (param to folder with json_payloads)
* functionality for A/B testing / testing different suites
* from json / yaml
* kaleido support? https://github.com/igiagkiozis/plotly#exporting-an-interactive-plot
* wasm support? https://github.com/igiagkiozis/plotly#exporting-an-interactive-plot
* show also curve / timeseries
* add request_id to request, so that it can be traced back potentially? tbd: could be responsibility of user
