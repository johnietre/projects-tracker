PRAGMA foreign_keys=OFF;
PRAGMA foreign_keys=ON;
PRAGMA recursive_triggers=ON;

BEGIN TRANSACTION;

CREATE TABLE IF NOT EXISTS users (
  email VARCHAR(255) PRIMARY KEY,
  --password_hash TEXT NOT NULL
  password_hash VARCHAR(255) NOT NULL
);

--CREATE TRIGGER del_user_table AFTER DELETE ON users
--BEGIN
  --DROP TABLE IF EXISTS [johnietrebus@gmail.com];
--END;

INSERT INTO users VALUES ("johnietrebus@gmail.com","1");

CREATE TABLE IF NOT EXISTS [johnietrebus@gmail.com] (
  part_id INTEGER PRIMARY KEY AUTOINCREMENT,
  name VARCHAR(255) NOT NULL,
  --description TEXT,
  description VARCHAR(255),
  deadline INTEGER,
  completed_at INTEGER,
  parent_id INTEGER,
  FOREIGN KEY(parent_id) REFERENCES [johnietrebus@gmail.com](part_id) ON DELETE CASCADE
);

COMMIT;

CREATE TRIGGER [update_johnietrebus@gmail.com]
  AFTER UPDATE OF completed_at ON [johnietrebus@gmail.com]
  WHEN NEW.parent_id IS NOT NULL
    --AND (OLD.completed_at <> NEW.completed_at OR NEW.completed_at IS NOT NULL)
BEGIN
  UPDATE [johnietrebus@gmail.com]
    SET completed_at=NEW.completed_at
    WHERE completed_at IS NULL
      AND part_id=NEW.parent_id;
      --AND (
        --SELECT MAX(completed_at) FROM [johnietrebus@gmail.com] WHERE parent_id=NEW.parent_id
      --)=NEW.parent_id;
END;

INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Project 1",NULL,NULL);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Project 2","The second project",NULL);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Project 1",1);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 2","Part 2 of Project 1",1);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Project 2",2);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Part 1 of Project 2",5);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 2","Part 2 of Part 1 of Project 2",5);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Part 2 of Part 1 of Project 2",7);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 2","Part 2 of Project 2",2);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Project 3","Third time's a charm",NULL);

INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Project 4","The fourth project",NULL);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Project 4",11);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Part 1 of Project 4",12);
INSERT INTO [johnietrebus@gmail.com](name,description,parent_id) VALUES ("Part 1","Part 1 of Part 1 of Part 1 of Project 4",13);
