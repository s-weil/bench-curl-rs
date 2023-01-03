![Burl-rs](./resources/burl_logo.svg)

<b>Burl</b> is a statistics-driven benchmarking tool and load generator for http-webapplications, written in Rust.
Measure KPIs of your endpoints, compare different versions of them and analyze the performance with the provided reports, containing plots and result data.
Run Burl as CLI tool and manage your setup conveniently with config files (see [config](./examples/actix/specs.toml)).

### Examples
Find sample reports for 
* [FastApi (Python)](https://fastapi.tiangolo.com/): [config](./examples/actix/actix_specs.toml) [report](./examples/fastapi/report/report.html)
* [Actix web (Rust)](https://github.com/actix/actix-web): [config](./examples/fastapi/fastapi_specs.toml) [report](./examples/actix/report/report.html)


<img src="./resources/console_summary.png" width="300" height="300" /><br>
<img src="./resources/durations_box_plot.png" width="600" height="300" /><br>
<img src="./resources/durations_histogram.png" width="600" height="300" /><br>
<img src="./resources/durations_timeseries.png" width="600" height="300" />


### TODO:
* iteration support for proper benchmarking
* split config
* cli improvement and clarification
* stats-extension:
    * add console output to report.html
    * reuse (baseline) results for comparison  
    * confidence interval
    * analyze outliers
    * bivariate metrics for comparison of runs
* reuse connections / persistence, see e.g. https://twitter.com/mlafeldt/status/1437411223948103683 or https://users.rust-lang.org/t/hyper-reqwest-connection-not-being-kept-alive/10895/5
* error handling
* -> support jupyter notebooks! via python api
* input randomizer (param to folder with json_payloads)
* functionality for A/B testing / testing different suites
* from json / yaml
* kaleido support? https://github.com/igiagkiozis/plotly#exporting-an-interactive-plot
* wasm support? https://github.com/igiagkiozis/plotly#exporting-an-interactive-plot
* add request_id to request, so that it can be traced back potentially? tbd: could be responsibility of user
* add lib error
* platform builds, incl docker
