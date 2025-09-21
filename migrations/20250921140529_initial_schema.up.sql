CREATE TABLE users (
  id UUID PRIMARY KEY,
  email TEXT UNIQUE NOT NULL,
  username TEXT NOT NULL,
  password_hash TEXT,
  email_verified_at TIMESTAMPTZ,
  available_charges INTEGER NOT NULL DEFAULT 30,
  charges_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT users_email_chk CHECK (position('@' in email) > 1),
  CONSTRAINT users_username_unique UNIQUE (username)
);

CREATE INDEX idx_users_charges_updated_at ON users(charges_updated_at);

CREATE TABLE user_identities (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  provider TEXT NOT NULL,
  provider_user_id TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (provider, provider_user_id)
);

CREATE TABLE pixel_history (
    user_id UUID NOT NULL REFERENCES users(id),
    global_x INTEGER NOT NULL,
    global_y INTEGER NOT NULL,
    color_id SMALLINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (global_x, global_y)
);

CREATE INDEX idx_pixel_history_user_id ON pixel_history(user_id);

CREATE INDEX idx_pixel_history_global_coords ON pixel_history(global_x, global_y);

CREATE INDEX idx_pixel_history_global_x ON pixel_history(global_x);
CREATE INDEX idx_pixel_history_global_y ON pixel_history(global_y);

CREATE INDEX idx_pixel_history_created_at ON pixel_history(created_at DESC);

CREATE TABLE email_verification_tokens (
    token TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);
