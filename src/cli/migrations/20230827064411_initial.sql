begin;

create extension if not exists timescaledb cascade;

create table measurements (
  id bigserial,
  source text not null,
  timestamp timestamp with time zone not null,
  data jsonb not null,
  primary key (id, source, timestamp)
);
select create_hypertable('measurements', 'timestamp');

create type log_kind as enum ('success', 'failure');
create table logs (
  id bigserial primary key not null,
  timestamp timestamp with time zone not null,
  -- foreign keys to hypertables are not supported
  last_measurement bigserial not null,
  kind log_kind not null,
  response jsonb not null
);

commit;
