# HamTTPD

Encryption-less web service for amateur radio using TLS client certificates for authentication

International bodies and local governments grants special rights to operators of the [amateur radio service](https://en.wikipedia.org/wiki/Amateur_radio).
This allows free tinkering and experimentation with wireless technology.
In exchange, all communications should be performed [in the clear](https://qsantos.fr/2022/12/21/ham-crypto/).

However, this does not preclude the use of cryptography altogether.
With [client authentication](https://blog.cloudflare.com/introducing-tls-client-auth/), the service can still allow visitors to log in.

See also [HamFox](https://github.com/qsantos/hamfox) and [HamSSH](https://github.com/qsantos/hamssh).

# Installation

Install the set-misc module for Nginx.
If it is included in your distribution, the package's names might be `nginx-plus-module-set-misc`.
For more detailed instructions, go to <https://docs.nginx.com/nginx/admin-guide/dynamic-modules/set-misc/>.

You can then set up a reverse proxy with the provided `nginx.conf`, and start the service with `cargo run`.

# Creating a Local CA

To allow visitors to authenticate with test certificates, you need to set up a local certificate authority.
This is done by generate a private cryptographic key and a certificate with the commands below.

```
$ openssl genrsa -out ca.key 4096
$ openssl req -new -x509 -key ca.key -out ca.pem -subj "/CN=localhost"
```

The file `ca.pem` should then be included in the file that is pointed to by `ssl_client_certificate` in the Nginx configuration file.
The file `ca.key` should be located in directory from which the service is run, to allow it to create new client certificates.
