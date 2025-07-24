# RustFinger
RustFinger is an extremely light weight, yet high performance WebFinger server written in Rust. The design revolves around providing a service which has near zero performance requirements, allowing users to utilize WebFinger on even the smallest devices.


So why RustFinger?\
Some services require WebFinger for authentication, even more so within the OIDC space. While there are WebFinger servers out there, we noticed a lack of something that was important to our operations - multitenancy.

RustFinger solves this issue by allowing you to configure multiple tenancies - in multiple ways.\
We also do strongly prefer containers, and as such the server has been written for container first, with the intention of the server running behind a reverse proxy.

### Performance
Of course, performance is always on everyone's mind - and as you can imagine with what's been mentioned, it was on ours as well.\
We want to provide the ability for anyone to use WebFinger for whatever project they may need it for. So here's some metrics:
- **Memory Usage**: 1.4MB (idle) / 2.2MB (>10k rps)
- **CPU Usage**: 0.0 (idle) / 0.0 (load)
- **Average Response Time**: <1ms
- **Docker Image Size**: ~18MB

#### Load Handling & Philosophy
- Designed for >10k requests per second in mind
- Async architecture
- Zero-allocation JSON responses

## Setup
As noted, this was built with containers in mind and has not been tested beyond it.


Building the image is very simple:
```
git clone https://github.com/Mimmikk/RustFinger
cd RustFinger
docker build -t rustfinger .
docker compose up -d
```

You're now prepared.\
We suggest using (and modifying) the `docker-compose.yml` in `examples/` for the best experience

### Container Health
Container runs health under `/healtz`, if needed.

## Configuration
Configuration is done with YAML files in the `config/` directory. Each `.yml` file defines a tenant.

### Example Configuration (`config/example.yml`)

```yaml
mysite:
    domain: "mysite.com"
    users:
        user1@mysite.com:
            name: "First User"
            avatar: "https://mysite.com/user1-pic"
            openid: "https://auth.mysite.com"
        user2@mysite.com:
            name: "Other User"
            openid: "https://sso.mysite.com"

othersite:
    domain: "othersite.com"
    openid: "https://auth.othersite.com"
    global: true
```

### Configuration Options

- **Tenant Name**: The first level key in the YAML (e.g., `mysite`)
- `domain`: Required when used behind a reverse proxy
- `users`: Map of user identifiers to their WebFinger data
- `global`: If true, accepts any user for the domain (use with caution)
- `openid`: OpenID Connect issuer URL

### URN Aliases (`urns.yml`)

Maps short names to standard WebFinger URNs:

```yaml
name: "http://schema.org/name"
avatar: "http://webfinger.net/rel/avatar"
openid: "http://openid.net/specs/connect/1.0/issuer"
```

## Architecture

RustFinger is built with:

- **Axum**: Ultra-fast HTTP framework with minimal overhead
- **Tokio**: Efficient async runtime
- **Serde**: Zero-copy JSON serialization
- **Minimal Dependencies**: Only essential crates for maximum performance