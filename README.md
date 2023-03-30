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
