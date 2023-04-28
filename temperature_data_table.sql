CREATE TABLE temperature_data(
    locID INT,
    timestamp TIMESTAMP,
    temperature FLOAT,
    humidity FLOAT
);

CREATE TABLE location_names(
    name VARCHAR(255),
    locID INT NOT NULL AUTO_INCREMENT,
    PRIMARY KEY (locID)
);
