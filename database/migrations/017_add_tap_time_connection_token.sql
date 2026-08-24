-- The token is returned once when a user redeems a Tap-Time connection code.
-- It is encrypted by the Goddard backend before it is stored here.
ALTER TABLE tap_time_connections
    ADD COLUMN IF NOT EXISTS access_token_ciphertext BYTEA,
    ADD COLUMN IF NOT EXISTS access_token_nonce BYTEA;
