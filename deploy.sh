set -eou pipefail

docker build . -t jackratner/drum-circle-server

docker push jackratner/drum-circle-server
