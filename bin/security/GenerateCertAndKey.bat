@echo off

rem Generate Self Signed Cert (intended for quiche) (needs openssl 3.0+)
"C:\Program Files\Git\mingw64\bin\openssl.exe" req -x509 -noenc -newkey rsa:2048 -outform PEM -keyout pkey.pem -out cert.pem -sha256 -days 3650 -subj "/C=US/ST=NewYork/L=NYC/O=MediaEnhanced/OU=Swiftlet/CN=localhost" -config CertConfig.cnf

pause
exit 0
