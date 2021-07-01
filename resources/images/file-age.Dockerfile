FROM alpine

# latest alpine image seems to use an outdated version of 'find' that does not support '-printf' so update it
RUN apk -U add findutils