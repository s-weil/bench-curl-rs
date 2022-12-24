# bURL (benchmarking-curl-rs)

Statistics-driven benchmarking and load generation of http-webapplications, in Rust.

<img src="./examples/box_plot.jpg" width="700" height="600" />
<img src="./examples/durations_timeseries.png" width="700" height="500" />


### TODO:
* iteration support for proper benchmarking
* split config
* stats-extension:
    * dump also a table of stats and the raw results into the results folder 
    * plot confidence interval
* tokio support (tbd)
* rayon support
* -> support jupityer notebooks! via python api
* input randomizer (param to folder with json_payloads)
* functionality for A/B testing / testing different suites
* from json / yaml
* kaleido support? https://github.com/igiagkiozis/plotly#exporting-an-interactive-plot
* wasm support? https://github.com/igiagkiozis/plotly#exporting-an-interactive-plot
* add request_id to request, so that it can be traced back potentially? tbd: could be responsibility of user
* add lib error
