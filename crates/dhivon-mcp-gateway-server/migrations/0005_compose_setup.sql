-- Empty on manually bootstrapped deployments.
CREATE TABLE compose_setup (
    singleton boolean PRIMARY KEY CHECK (singleton),
    proof jsonb NOT NULL
);
