use std::borrow::Cow;

use crate::core::{
    auth::{hardware::HardwareHashError, pc_sign::PCSign, storage::AuthError},
    clients::JUNO_PC_CLIENT_ID,
    endpoints::API_NUCLEUS_AUTH,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::random;
use ring::hmac::HMAC_SHA256;
use sha2_const::Sha256;

/// Context with utilities for auth flow
pub struct AuthContext<'a> {
    code_verifier: String,
    code_challenge: String,
    code: Option<String>,
    scopes: Vec<String>,
    access_token: Option<String>,
    token_format: Option<String>,
    expires_in: Option<i64>,
    pc_sign: PCSign<'a>,
}

impl AuthContext<'_> {
    pub fn new() -> Result<Self, AuthError> {
        let verifier = Self::generate_code_verifier();
        let challenge = Self::generate_challenge(&verifier);
        let signature = PCSign::new()?;

        Ok(Self {
            code_verifier: verifier,
            code_challenge: challenge,
            code: None,
            scopes: Vec::new(),
            access_token: None,
            token_format: None,
            expires_in: None,
            pc_sign: signature,
        })
    }

    fn generate_code_verifier() -> String {
        let rand_bytes: [u8; 32] = random();
        URL_SAFE_NO_PAD.encode(&rand_bytes)
    }

    fn generate_challenge(code_verifier: &String) -> String {
        let hash = Sha256::new().update(code_verifier.as_bytes()).finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }

    pub fn generate_pc_sign(&self) -> Result<String, HardwareHashError> {
        let json_formatted_sign = serde_json::to_string(&self.pc_sign)?;
        let payload = URL_SAFE_NO_PAD.encode(json_formatted_sign.as_bytes());

        let key = ring::hmac::Key::new(HMAC_SHA256, self.pc_sign.sign_key());
        let value = ring::hmac::sign(&key, payload.as_bytes());

        Ok(payload.to_string() + "." + URL_SAFE_NO_PAD.encode(value).as_ref())
    }

    pub fn code_verifier(&self) -> &str {
        &self.code_verifier
    }

    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    pub fn add_scope(&mut self, scope: &str) {
        self.scopes.push(scope.to_owned());
    }

    pub fn set_code(&mut self, code: &str) {
        self.code = Some(code.to_owned())
    }

    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    pub fn set_access_token(&mut self, token: &str) {
        self.access_token = Some(token.to_owned())
    }

    pub fn token_format(&self) -> Option<&str> {
        self.token_format.as_deref()
    }

    pub fn set_token_format(&mut self, token_format: &str) {
        self.token_format = Some(token_format.to_owned());
    }

    pub fn expires_in(&self) -> Option<i64> {
        self.expires_in
    }

    pub fn set_expires_in(&mut self, expires_in: i64) {
        self.expires_in = Some(expires_in);
    }

    pub fn nucleus_auth_url(
        &self,
        client_id: &str,
        response_type: &str,
    ) -> Result<String, AuthError> {
        let signature = self.generate_pc_sign()?;
        let nonce = random::<i32>().to_string();

        let mut query = vec![
            ("client_id", Cow::Borrowed(client_id)),
            ("sbiod_enabled", Cow::Borrowed("false")),
            ("response_type", Cow::Borrowed(response_type)),
            ("locale", Cow::Borrowed("en_US")),
            ("pc_sign", Cow::Borrowed(&signature)),
            ("nonce", Cow::Borrowed(&nonce)),
        ];

        let scopes = self.scopes.join(" ");
        if !scopes.is_empty() {
            query.push(("scope", Cow::Owned(scopes)));
        }

        if client_id == JUNO_PC_CLIENT_ID {
            query.push(("code_challenge_method", Cow::Borrowed("S256")));
            query.push(("code_challenge", Cow::Borrowed(&self.code_challenge)));
        }

        if let Some(access_token) = &self.access_token {
            query.push(("access_token", Cow::Borrowed(access_token)));
        }

        if let Some(token_format) = &self.token_format {
            query.push(("token_format", Cow::Borrowed(token_format)));
        }

        if let Some(expires_in) = self.expires_in {
            query.push(("expires_in", Cow::Owned(expires_in.to_string())));
        }

        let url = reqwest::Url::parse_with_params(API_NUCLEUS_AUTH, query)?;
        Ok(url.to_string())
    }
}
