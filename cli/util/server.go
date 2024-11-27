/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package util

import (
	"fmt"
	"log"
	"net"
	"os"

	"github.com/skeema/knownhosts"
	"golang.org/x/crypto/ssh"
)

func NewSSHClient(host string, keypath string, hostsfile string) (*ssh.Client, error) {
	key, err := os.ReadFile(keypath)
	if err != nil {
		return nil, fmt.Errorf("unable to read private key: %v", err)
	}

	// Create the Signer for this private key.
	signer, err := ssh.ParsePrivateKey(key)
	if err != nil {
		return nil, fmt.Errorf("unable to parse private key: %v", err)
	}

	kh, err := knownhosts.New(hostsfile)
	if err != nil {
		return nil, fmt.Errorf("Error parsing hostfile %s: %v", hostsfile, err)
	}

	// Create a custom permissive hostkey callback which still errors on hosts
	// with changed keys, but allows unknown hosts and adds them to known_hosts
	cb := ssh.HostKeyCallback(func(hostname string, remote net.Addr, key ssh.PublicKey) error {
		err := kh(hostname, remote, key)
		if knownhosts.IsHostUnknown(err) {
			f, ferr := os.OpenFile(hostsfile, os.O_APPEND|os.O_WRONLY, 0600)
			if ferr == nil {
				defer f.Close()
				ferr = knownhosts.WriteKnownHost(f, hostname, remote, key)
			}
			if ferr == nil {
				log.Printf("Added host %s to known_hosts\n", hostname)
			} else {
				log.Printf("Failed to add host %s to known_hosts: %v\n", hostname, ferr)
			}
			return nil // permit previously-unknown hosts (warning: may be insecure)
		}
		return err
	})

	config := &ssh.ClientConfig{
		Auth: []ssh.AuthMethod{
			ssh.PublicKeys(signer),
		},
		HostKeyCallback:   cb,
		HostKeyAlgorithms: kh.HostKeyAlgorithms(host),
	}

	// Connect to SSH server
	conn, err := ssh.Dial("tcp", host, config)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to SSH server: %v", err)
	}

	return conn, nil
}
