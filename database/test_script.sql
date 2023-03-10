PRAGMA foreign_keys=off;

BEGIN TRANSACTION;

CREATE TABLE IF NOT EXISTS users (
  email VARCHAR PRIMARY KEY,
  password_hash TEXT NOT NULL
);

INSERT INTO users VALUES ("johnietrebus@gmail.com","1");

CREATE TABLE IF NOT EXISTS [johnietrebus@gmail.com] (
  part_id INTEGER PRIMAY KEY AUTOINCREMENT,
  VARCHAR TEXT NOT NULL,
  description TEXT,
  deadline VARCHAR,
  completed_at VARCHAR,
  parent_id INTEGER,
  FOREIGN KEY(parent_id) REFERENCES [johnietrebus@gmail.com](part_id)
);

INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Project 1",NULL,NULL);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Project 2","The second project",NULL);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Project 1",1);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 2","Part 2 of Project 1",1);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Project 2",2);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Part 1 of Project 2",5);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 2","Part 2 of Part 1 of Project 2",5);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Part 2 of Part 1 of Project 2",7);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 2","Part 2 of Project 2",2);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES (,"Project 3","Third time's a charm",NULL);

COMMIT;

PRAGMA foreign_keys=on;
