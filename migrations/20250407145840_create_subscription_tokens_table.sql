-- create subscription_tokens table
CREATE TABLE subscription_tokens (
    subscription_token TEXT NOT NULL,
    subscriber_id uuid NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    PRIMARY KEY (subscription_token)
);
