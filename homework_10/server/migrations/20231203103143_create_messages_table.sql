
CREATE TABLE users(
    id uuid NOT NULL,
    PRIMARY KEY (id),
    password TEXT NOT NULL,
    salt TEXT NOT NULL,
    username TEXT NOT NULL,
    last_login timestamptz NOT NULL
);

CREATE TABLE messages(
    id uuid NOT NULL,
    PRIMARY KEY (id),
    user_id uuid NOT NULL,
    data TEXT NOT NULL,
    timestamp timestamptz NOT NULL,
    CONSTRAINT fk_user
      FOREIGN KEY(user_id)
	  REFERENCES users(id)
      ON DELETE CASCADE
);

