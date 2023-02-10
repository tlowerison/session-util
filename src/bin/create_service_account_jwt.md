# create_service_account_jwt
Use this binary to create service account jwts to be provided as environment variables to services so that they can securely communicate with each other.

## Usage
```sh
export SESSION_JWT_PRIVATE_CERTIFICATE="<private certificate>"
export SESSION_JWT_PUBLIC_CERTIFICATE="<public certificate>"
export ACCOUNT_ID="<account id>"
export ROLE_ID="<role id>"

cargo run --bin create_service_account_jwt --features create_service_account_jwt
```
