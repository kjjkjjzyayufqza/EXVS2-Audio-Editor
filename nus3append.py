import sys
import os
import struct
import mmap

filepath = input("Enter the filepath to nus3bank: ")

def readu32le(file):
    return struct.unpack(b"<I", file.read(4))[0]

def get_sub_meta_size(file):
    while file.tell() % 4 != 0:
        file.seek(1, 1)
    while readu32le(file) != 0x22E8:
        pass
    start_pos = file.tell()
    break_counter = 0
    while True:
        val = readu32le(file)
        if break_counter % 2 == 0:
            if val == 0:
                break_counter += 1
            else:
                break_counter = 0
        else:
            if val == 0xFFFFFFFF:
                break_counter += 1
            else:
                break_counter = 0
        if break_counter == 8:
            break
    return file.tell() - start_pos

assert os.path.exists(filepath), "File not found: " + str(filepath)
nus3bank = open(filepath, u'rb+')
if nus3bank.read(4) != b"NUS3":
    print("Invalid file.")
    exit()
size = readu32le(nus3bank)
assert nus3bank.read(8) == b"BANKTOC ", u"Invalid file."
tocSize = readu32le(nus3bank)
contentCount = readu32le(nus3bank)
offset = 0x14 + tocSize
for i in range(contentCount):
    content = nus3bank.read(4)
    toneHeaderOffset = nus3bank.tell()
    contentSize = readu32le(nus3bank)
    if content == b"TONE":
        toneOffset = offset
        toneSize = contentSize
        break
    offset += 8 + contentSize
nus3bank.seek(toneOffset)
assert nus3bank.read(4) == b"TONE"
assert readu32le(nus3bank) == toneSize
toneCount = readu32le(nus3bank)
append_name = input("Enter the name of the appended sound effect: ")
assert str(append_name), u"Invalid sound effect name"
comparable_name = input("Enter the name of a comparable sound effect (metadata): ")
assert str(comparable_name), u"Invalid sound effect name"
file_as_string = mmap.mmap(nus3bank.fileno(), 0, access=mmap.ACCESS_READ)
comparable_offset = file_as_string.find(bytes(comparable_name, 'utf8'), toneOffset)
assert comparable_offset != -1, u"Comparable sound effect not found in this file. Are you sure you spelled it right?"
nus3bank.seek(comparable_offset)
comparable_size = get_sub_meta_size(nus3bank)
nus3bank.seek(comparable_offset)
val = readu32le(nus3bank)
while nus3bank.tell() % 4 != 0:
    nus3bank.seek(1, 1)
while readu32le(nus3bank) != 0x22E8:
    pass
comparable_meta_data = nus3bank.read(comparable_size)
nus3bank.seek(comparable_offset - 0xD)
comparable_pre_meta_data = nus3bank.read(0xC)
nus3bank.seek(toneOffset + 12 + (toneCount - 1) * 8)
lastToneOffset = readu32le(nus3bank)
lastToneSize = readu32le(nus3bank)
newToneSize = comparable_size + 28 + len(append_name) + 1
newToneSize += 4 - ((len(append_name) + 1) % 4)
nus3bank.seek(0)
original_file = nus3bank.read()
nus3bank.close()
nus3bank = open(filepath, u'rb+')
nus3bank.seek(0)
nus3bank.write(b"NUS3")
nus3bank.write(struct.pack(b"<I", size + newToneSize + 8))
nus3bank.write(original_file[8:toneHeaderOffset])
nus3bank.write(struct.pack(b"<I", toneSize + newToneSize + 8))
nus3bank.write(original_file[(toneHeaderOffset + 4):(toneOffset + 4)])
nus3bank.write(struct.pack(b"<I", toneSize + newToneSize + 8))
modified_count = struct.pack(b"<I", toneCount + 1);
nus3bank.write(modified_count)
counter = 0
while counter < toneCount:
    curToneOffset = struct.unpack(b"<I", original_file[(toneOffset + 12 + counter * 8):(toneOffset + 16 + counter * 8)])[0]
    modToneOffset = struct.pack(b"<I", curToneOffset + 8)
    nus3bank.write(modToneOffset)
    nus3bank.write(original_file[(toneOffset + 16 + counter * 8):(toneOffset + 20 + counter * 8)])
    counter += 1
nus3bank.write(struct.pack(b"<I", lastToneOffset + lastToneSize + 8))
nus3bank.write(struct.pack(b"<I", newToneSize))
nus3bank.write(original_file[(toneOffset + 12 + toneCount * 8):(toneOffset + 8 + lastToneOffset + lastToneSize)])
nus3bank.write(comparable_pre_meta_data);
nus3bank.write(struct.pack(b"B", len(append_name) + 1))
nus3bank.write(bytes(append_name, 'utf8'))
counter = len(append_name) + 1
if counter % 4 == 0:
    nus3bank.write(struct.pack(b"<I", 0))
while counter % 4 != 0:
    nus3bank.write(struct.pack(b"B", 0))
    counter += 1
nus3bank.write(struct.pack(b"<I", 0))
nus3bank.write(struct.pack(b"<I", 0x8))
nus3bank.write(struct.pack(b"<I", 0))
nus3bank.write(struct.pack(b"<I", 0x22E8))
nus3bank.write(comparable_meta_data)
nus3bank.write(original_file[(toneOffset + 8 + lastToneOffset + lastToneSize):])
nus3bank.close()
extension_index = filepath.find('.')
extension_count = len([f for f in os.listdir(os.path.dirname(os.path.abspath(filepath))) if f.endswith(filepath[extension_index:]) and os.path.isfile(os.path.join(os.path.dirname(os.path.abspath(filepath)), f))])
if extension_index == -1:
    nus3bank = open(filepath + "_backup_" + str(extension_count), u'wb+')
else:
    nus3bank = open(filepath[:extension_index] + "_backup_" + str(extension_count) + filepath[extension_index:], u'wb+')
nus3bank.write(original_file)
nus3bank.close()
print("Added entry", toneCount, "to", filepath, "uwu")
