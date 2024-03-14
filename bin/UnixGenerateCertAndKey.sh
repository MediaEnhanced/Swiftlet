#!/bin/sh
# Generate a Self Signed Cert (currently intended for Rust quiche library)
# openssl 3.0+

openssl req -x509 -noenc -newkey rsa:2048 -outform PEM -config security/CertConfig.cnf -keyout security/pkey.pem -out security/cert.pem -sha256 -days 3650 -subj "/C=US/ST=NewYork/L=NYC/O=MediaEnhanced/OU=Swiftlet/CN=localhost"

exit 0
