CREATE TABLE banned_users (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    banned_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    reason TEXT NOT NULL,
    banned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT banned_users_user_id_unique UNIQUE (user_id)
);

CREATE INDEX idx_banned_users_user_id ON banned_users(user_id);
CREATE INDEX idx_banned_users_banned_at ON banned_users(banned_at DESC);
CREATE INDEX idx_banned_users_expires_at ON banned_users(expires_at) WHERE expires_at IS NOT NULL;
