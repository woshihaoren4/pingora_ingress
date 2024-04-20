#!/bin/bash

function build_docker_image() {
  echo "start compile application..."
  if [ ! -e ".cargo/config.toml" ] ; then
    echo "mkdir .cargo;touch .cargo/config.toml"
    mkdir .cargo;touch .cargo/config.toml
  fi
  cargo_config_content='
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"'
  echo "$cargo_config_content" > .cargo/config.toml
  echo -e "write cargo config file :\n<---$cargo_config_content\n<---"

  compile_cmd="cargo build --release --target=x86_64-unknown-linux-musl"
  echo $compile_cmd
  ${compile_cmd}
  echo "compile success"

  echo "rm -rf .cargo"
  rm -rf .cargo
  echo "cargo config file clear success."

  echo "start build image use docker..."
  if [ -n "$2" ]; then
      version=$2
  else
    timestamp=$(date +%s%N)
    version=${timestamp:2:8}
  fi
  tag="wdshihaoren/pingora-ingress:$version"
  build_cmd="docker build -f ./Dockerfile -t $tag ./"
  echo $build_cmd
  ${build_cmd}

  echo "docker build success image:$tag"
  docker_push_cmd="docker push $tag"
  echo $docker_push_cmd
  ${build_cmd}

  echo "docker push success"
}

case $1 in
docker)
  build_docker_image "$@"
  ;;
esac

