@echo off

rem Generate a Self Signed Cert (currently intended for Rust quiche library)
rem Expects Git to be installed (and for it to contain openssl 3.0+)
"C:\Program Files\Git\mingw64\bin\openssl.exe" req -x509 -noenc -newkey rsa:2048 -outform PEM -config security/CertConfig.cnf -keyout security/pkey.pem -out security/cert.pem -sha256 -days 3650 -subj "/C=US/ST=NewYork/L=NYC/O=MediaEnhanced/OU=Swiftlet/CN=localhost"

pause
exit 0
