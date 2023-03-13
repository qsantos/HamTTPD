CREATE TABLE message (
    id SERIAL PRIMARY KEY,
    created TIMESTAMP NOT NULL DEFAULT now(),
    author TEXT NOT NULL,
    content TEXT NOT NULL
)