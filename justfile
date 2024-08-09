alias f := fix
alias t := test 
alias to := test_no_capture

fix:
  cargo fix --allow-dirty
  cargo clippy --fix --allow-dirty 

docker_clean: 
  - docker rm $(docker ps -aq)
  - docker kill $(docker ps -aq)
  - docker image rm $(docker image list -aq)

docker_prune:
  - docker rm -f $(docker ps -aq)
  - echo y | docker network prune

test *ARGS: docker_prune
  - cargo nextest run {{ARGS}}

test_no_capture *ARGS: docker_prune
  @ just test --nocapture {{ARGS}}

