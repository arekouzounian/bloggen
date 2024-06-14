cp $HOME/.ssh/authorized_keys ./server/ && docker compose build 
rm ./server/authorized_keys