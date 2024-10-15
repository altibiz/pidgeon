begin;

alter table devices
  alter column address drop not null,
  add column path text null,
  add column baud_rate int null;

commit;
