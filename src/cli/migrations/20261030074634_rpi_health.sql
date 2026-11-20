-- Add migration script here
begin;

-- TODO: columns marked "not collected yet" are not yet collected -- TODO
create table system_health (

  timestamp timestamp with time zone not null primary key,
  boot_id text not null,
  uptime_seconds bigint not null,

  -- thermal & power
  cpu_temperature_celsius real null,
  cpu_frequency_mhz integer null,
  throttled_flags integer null,

  -- cpu
  cpu_usage_percent real null,
  cpu_iowait_percent real null,
  load_1m real null,
  load_5m real null,
  load_15m real null,

  -- memory
  memory_total_bytes bigint null,
  memory_available_bytes bigint null,
  swap_used_bytes bigint null,
  oom_kills bigint null,
  pidgeon_rss_bytes bigint null,

  -- storage
  rootfs_total_bytes bigint null,
  rootfs_free_bytes bigint null,
  rootfs_free_inodes bigint null,
  fs_errors bigint null,
  fs_lifetime_written_kbytes bigint null,
  disk_reads bigint null, -- not collected yet
  disk_read_ms bigint null, -- not collected yet
  disk_writes bigint null, -- not collected yet
  disk_write_ms bigint null, -- not collected yet
  disk_sectors_written bigint null, -- not collected yet
  database_bytes bigint null,

  -- network
  eth0_carrier_changes bigint null, -- not collected yet
  eth0_rx_errors bigint null, -- not collected yet
  eth0_tx_errors bigint null, -- not collected yet
  wlan0_rx_errors bigint null, -- not collected yet
  wlan0_tx_errors bigint null, -- not collected yet
  wlan0_signal_dbm real null, -- not collected yet

  -- kernel
  kernel_errors bigint null, -- not collected yet

  -- metrics not yet promoted to columns
  extra jsonb not null default '{}' -- not collected yet, stays '{}'
);
select create_hypertable('system_health', 'timestamp');

alter table system_health set (timescaledb.compress);
select add_compression_policy('system_health', interval '7 days');
select add_retention_policy('system_health', interval '1 month');

commit;
