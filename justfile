alias f := fix
alias t := test 
alias to := test_no_capture

fix:
  cargo fix --allow-dirty
  cargo clippy --fix --allow-dirty 

docker_clean: 
  ! docker rm $(docker ps -aq)
  ! docker kill $(docker ps -aq)
  ! docker image rm $(docker image list -aq)

test *ARGS: 
  cargo nextest run {{ARGS}}

test_no_capture: 
  @ just test --nocapture

