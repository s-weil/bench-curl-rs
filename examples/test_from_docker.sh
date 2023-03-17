#docker build burl:latest .
    
# --network=host: container able to connect to localhost / docker host
docker run \
    --network=host \
    burl
    # burl parameters

# mount your working dir (containing configs) to the local dir in the container
docker run \
    -v `pwd`:/app/target/release/local \
    --network=host \
    burl
    # burl parameters

# -it: run interactively: adjust config in file, etc.
# once started: run 
# `nano config/specs.toml`
docker run \
    -it \
    --network=host \
    burl bash
