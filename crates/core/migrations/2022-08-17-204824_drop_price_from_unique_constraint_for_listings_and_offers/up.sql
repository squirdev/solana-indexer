DO $$
BEGIN
  	IF (SELECT EXISTS(select * from pg_catalog.pg_constraint where conname = 'listings_unique_fields' and conkey = ARRAY[2, 3, 4, 5, 7, 8, 9]::smallint[])) THEN
		RAISE NOTICE 'excluding price from listings unique constraint';
		RAISE NOTICE 'ACQURING ACCESS EXCLUSIVE LOCK';
		BEGIN
		LOCK TABLE LISTINGS IN ACCESS EXCLUSIVE MODE;

		CREATE TEMP TABLE TEMP_DISTINCT_LISTINGS_IDS AS
		SELECT DISTINCT ON (TRADE_STATE, AUCTION_HOUSE, SELLER, METADATA, TOKEN_SIZE, TRADE_STATE_BUMP) ID
		FROM
			(SELECT *
				FROM LISTINGS
				ORDER BY CREATED_AT DESC) AS LISTING;

		ALTER TABLE TEMP_DISTINCT_LISTINGS_IDS ADD PRIMARY KEY (ID);


		CREATE TEMP TABLE LISTING_IDS AS
		SELECT A.ID
		FROM LISTINGS A
		LEFT JOIN TEMP_DISTINCT_LISTINGS_IDS B ON A.ID = B.ID
		WHERE B.ID IS NULL;

		DELETE
		FROM LISTINGS
		WHERE ID in
				(SELECT ID
					FROM LISTING_IDS);

		ALTER TABLE LISTINGS
		DROP CONSTRAINT LISTINGS_UNIQUE_FIELDS,
		ADD CONSTRAINT LISTINGS_UNIQUE_FIELDS UNIQUE (TRADE_STATE, AUCTION_HOUSE, SELLER, METADATA, TOKEN_SIZE, TRADE_STATE_BUMP);
		END;
		
		RAISE NOTICE 'LOCK RELEASED';
		
		DROP TABLE TEMP_DISTINCT_LISTINGS_IDS;
		DROP TABLE LISTING_IDS;

		CREATE TABLE LISTING_EVENTS_IDS AS
			SELECT *
			FROM LISTING_EVENTS LE
			LEFT JOIN LISTINGS L ON L.ID = LE.LISTING_ID
			WHERE L.ID IS NULL;

		RAISE NOTICE 'DELETING DUPLICATE LISTING EVENTS';

		DELETE
		FROM LISTING_EVENTS
		WHERE LISTING_EVENTS.FEED_EVENT_ID in
				(SELECT FEED_EVENT_ID
					FROM LISTING_EVENTS_IDS);

		RAISE NOTICE 'DELETING DUPLICATE FEED EVENT WALLETS';
		
		DELETE
		FROM FEED_EVENT_WALLETS
		WHERE FEED_EVENT_WALLETS.FEED_EVENT_ID in
				(SELECT FEED_EVENT_ID
					FROM LISTING_EVENTS_IDS);

		RAISE NOTICE 'DELETING DUPLICATE FEED EVENTS';

		DELETE
		FROM FEED_EVENTS
		WHERE FEED_EVENTS.ID in
				(SELECT FEED_EVENT_ID
					FROM LISTING_EVENTS_IDS);

		DROP TABLE LISTING_EVENTS_IDS;
	
	END IF;

    IF (SELECT EXISTS(select * from pg_catalog.pg_constraint where conname = 'offers_unique_fields' and conkey = ARRAY[2, 3, 4, 5, 8, 9, 10]::smallint[])) THEN
		RAISE NOTICE 'excluding price from offers unique constraint';
		RAISE NOTICE 'ACQURING ACCESS EXCLUSIVE LOCK';
		BEGIN
		LOCK TABLE OFFERS IN ACCESS EXCLUSIVE MODE;

		CREATE TEMP TABLE TEMP_DISTINCT_OFFERS_IDS AS
		SELECT DISTINCT ON (TRADE_STATE, AUCTION_HOUSE, BUYER, METADATA, TOKEN_SIZE, TRADE_STATE_BUMP) ID
		FROM
			(SELECT *
				FROM OFFERS
				ORDER BY CREATED_AT DESC) AS OFFER;

		ALTER TABLE TEMP_DISTINCT_OFFERS_IDS ADD PRIMARY KEY (ID);


		CREATE TEMP TABLE OFFER_IDS AS
		SELECT A.ID
		FROM OFFERS A
		LEFT JOIN TEMP_DISTINCT_OFFERS_IDS B ON A.ID = B.ID
		WHERE B.ID IS NULL;

		DELETE
		FROM OFFERS
		WHERE ID in
				(SELECT ID
					FROM OFFER_IDS);

		ALTER TABLE OFFERS
		DROP CONSTRAINT OFFERS_UNIQUE_FIELDS,
		ADD CONSTRAINT OFFERS_UNIQUE_FIELDS UNIQUE (TRADE_STATE, AUCTION_HOUSE, BUYER, METADATA, TOKEN_SIZE, TRADE_STATE_BUMP);
		END;
		
		RAISE NOTICE 'LOCK RELEASED';
		
		DROP TABLE TEMP_DISTINCT_OFFERS_IDS;
		DROP TABLE OFFER_IDS;

		CREATE TABLE OFFERS_EVENTS_IDS AS
			SELECT *
			FROM OFFER_EVENTS OE
			LEFT JOIN OFFERS O ON O.ID = OE.OFFER_ID
			WHERE O.ID IS NULL;

		RAISE NOTICE 'DELETING DUPLICATE OFFER EVENTS';

		DELETE
		FROM OFFER_EVENTS
		WHERE OFFER_EVENTS.FEED_EVENT_ID in
				(SELECT FEED_EVENT_ID
					FROM OFFERS_EVENTS_IDS);

		RAISE NOTICE 'DELETING DUPLICATE FEED EVENT WALLETS';
		
		DELETE
		FROM FEED_EVENT_WALLETS
		WHERE FEED_EVENT_WALLETS.FEED_EVENT_ID in
				(SELECT FEED_EVENT_ID
					FROM OFFERS_EVENTS_IDS);

		RAISE NOTICE 'DELETING DUPLICATE FEED EVENTS';

		DELETE
		FROM FEED_EVENTS
		WHERE FEED_EVENTS.ID in
				(SELECT FEED_EVENT_ID
					FROM OFFERS_EVENTS_IDS);

		DROP TABLE OFFERS_EVENTS_IDS;
	
	END IF;
END $$;

