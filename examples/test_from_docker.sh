#docker build burl:latest .
    
# --network=host: container able to connect to localhost / docker host
docker run \
    --rm \
    --network=host \
    burl
    # burl parameters

# mount your working dir (containing configs) to the local dir in the container
docker run \
    --rm \
    -v `pwd`:/app/data \
    --network=host \
    burl
    # burl parameters like `-f local/your_specs_file.toml from-toml`

# -it: run interactively: adjust config in file, etc.
# once started: run 
# `nano config/specs.toml`
docker run \
    --rm \
    -it \
    --network=host \
    --entrypoint /bin/bash \
    burl

# if your specs.toml file in your source dir is set up acordingly (`report_directory = "data/report"`),
# then your report will be available in the source dir after the run
docker run \
    --rm \
    -it \
    --network=host \
    --mount type=bind,source=`pwd`,target=/app/data \
    --entrypoint /bin/bash \
    burl


docker run --rm --network=host -v `pwd`:/app/data -it --entrypoint /bin/bash burl