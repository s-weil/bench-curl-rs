# bench-curl-rs

Statistics-driven benchmarking of http-webapp, in Rust.

<img src="./examples/box_plot.jpg" width="700" height="600" />


### TODO:
* warmup phase, only then requests
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
