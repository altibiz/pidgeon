begin;

-- NITPICK: https://github.com/timescale/timescaledb/issues/836

create extension if not exists timescaledb cascade;

create type device_status as enum ('healthy', 'unreachable', 'inactive');
create table devices (
  id text primary key not null,
  kind text not null,
  status device_status not null,
  seen timestamp with time zone not null,
  pinged timestamp with time zone not null,
  address inet not null,
  slave int null
);

create table health (
  id bigserial,
  source text not null,
  timestamp timestamp with time zone not null,
  status device_status not null,
  data jsonb not null,
  primary key (id, source, timestamp)
);
select create_hypertable('health', 'timestamp');

create table measurements (
  id bigserial,
  source text not null,
  timestamp timestamp with time zone not null,
  data jsonb not null,
  primary key (id, source, timestamp)
);
select create_hypertable('measurements', 'timestamp');

create type log_status as enum ('success', 'failure');
create type log_kind as enum ('push', 'update');
create table logs (
  id bigserial primary key not null,
  timestamp timestamp with time zone not null,
  last bigint null,
  kind log_kind not null,
  status log_status not null,
  response jsonb not null
);

commit;
