ALTER TABLE iap_orders
DROP CONSTRAINT IF EXISTS iap_orders_verify_mode_check;

ALTER TABLE iap_orders
ADD CONSTRAINT iap_orders_verify_mode_check
CHECK (verify_mode IN ('mock', 'apple'));
