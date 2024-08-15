create table if not exists links (
  id                         text not null primary key,
  target_url                 text not null,
  expiration                 timestamptz
);

create index idx_links_expiration on links using btree (expiration);