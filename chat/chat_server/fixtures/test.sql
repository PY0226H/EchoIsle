-- insert 5 users, all with hashed password '123456'
INSERT INTO users(email, fullname, password_hash)
  VALUES ('tchen@acme.org', 'Tyr Chen', '$argon2id$v=19$m=19456,t=2,p=1$MxGhY+ib/kplwBPLa7u2ug$c5h9u7Sc8Px8J5+qgNdOjSY7ZJO2QN4rugKpapGW4XU'),
('alice@acme.org', 'Alice Chen', '$argon2id$v=19$m=19456,t=2,p=1$MxGhY+ib/kplwBPLa7u2ug$c5h9u7Sc8Px8J5+qgNdOjSY7ZJO2QN4rugKpapGW4XU'),
('bob@acme.org', 'Bob Chen', '$argon2id$v=19$m=19456,t=2,p=1$MxGhY+ib/kplwBPLa7u2ug$c5h9u7Sc8Px8J5+qgNdOjSY7ZJO2QN4rugKpapGW4XU'),
('charlie@acme.org', 'Charlie Chen', '$argon2id$v=19$m=19456,t=2,p=1$MxGhY+ib/kplwBPLa7u2ug$c5h9u7Sc8Px8J5+qgNdOjSY7ZJO2QN4rugKpapGW4XU'),
('daisy@acme.org', 'Daisy Chen', '$argon2id$v=19$m=19456,t=2,p=1$MxGhY+ib/kplwBPLa7u2ug$c5h9u7Sc8Px8J5+qgNdOjSY7ZJO2QN4rugKpapGW4XU');

-- insert 4 chats
-- insert public/private channel
INSERT INTO chats(name, type, members)
  VALUES ('general', 'public_channel', '{1,2,3,4,5}'),
('private', 'private_channel', '{1,2,3}');

-- insert unnamed chat
INSERT INTO chats(type, members)
  VALUES ('single', '{1,2}'),
('group', '{1,3,4}');

-- insert agent to chat
INSERT INTO chat_agents(chat_id, name, type, adapter, model, prompt, args)
  VALUES (1, 'translation', 'proxy', 'test', 'gpt-4o', 'If language is Chinese, translate to English, if language is English, translate to Chinese. Please reply with the translated content directly. No explanation is needed. Here is the content: ', '{}');

INSERT INTO messages(chat_id, sender_id, content)
  VALUES (1, 1, 'Hello, world!'),
(1, 2, 'Hi, there!'),
(1, 3, 'How are you?'),
(1, 4, 'I am fine, thank you!'),
(1, 5, 'Good to hear that!'),
(1, 1, 'Hello, world!'),
(1, 2, 'Hi, there!'),
(1, 3, 'How are you?'),
(1, 1, 'Hello, world!'),
(1, 1, 'Hello, world!');
