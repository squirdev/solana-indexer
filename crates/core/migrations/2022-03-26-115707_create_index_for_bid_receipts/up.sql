create index open_offers_idx on bid_receipts (buyer, purchase_receipt, canceled_at) where buyer is null and canceled_at is null; 