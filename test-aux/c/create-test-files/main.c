#include <stdio.h>
#include <glib-2.0/glib.h>
#include "../gvdb/gvdb/gvdb-builder.h"
#include "../gvdb/gvdb/gvdb-reader.h"

#define TEST_PATH "../../../test-data/"
#define TEST_FILE_1 TEST_PATH "test1.gvdb"
#define TEST_FILE_2 TEST_PATH "test2.gvdb"

/**
 * Pretty prints a gvdb table structure
 * @param table The table to print
 * @param indent The indentation level
 */
void dump_gvdb_table(GvdbTable *table, int indent) {
    printf("%*s{\n", indent, "");
    gsize length;
    gchar **names = gvdb_table_get_names(table, &length);

    indent += 2;

    int i = 0;
    while (names[i] != NULL) {
        printf("%*s'%s': ", indent, "", names[i]);
        GVariant *variant = gvdb_table_get_value(table, names[i]);
        if (variant != NULL) {
            printf("%s\n", g_variant_print(variant, TRUE));
        } else {
            printf("\n");
            GvdbTable *sub_table = gvdb_table_get_table(table, names[i]);
            if (table != NULL) {
                dump_gvdb_table(sub_table, indent);
            }
        }

        i++;
    }

    printf("%*s}\n", indent - 2, "");
}

/**
 * The data stored in this file is equivalent to the following dict:
 * {
 *     "root_key": (uint32 1234, uint32 98765, 'TEST_STRING_VALUE'),
 * }
 *
 * Test file 1 is little endian
 */
void create_test_file_1() {
    printf("Creating test file 1\n");
    GHashTable *table = gvdb_hash_table_new(NULL, NULL);

    GvdbItem *item = gvdb_hash_table_insert(table, "root_key");
    GVariant *data = g_variant_new_parsed("(uint32 1234, uint32 98765, 'TEST_STRING_VALUE')");

    gvdb_item_set_value(item, data);

    GError *error = NULL;

    gvdb_table_write_contents(table, TEST_FILE_1, G_BYTE_ORDER != G_LITTLE_ENDIAN, &error);
}

void read_test_file_1() {
    GError *error = NULL;
    GvdbTable *table = gvdb_table_new(TEST_FILE_1, FALSE, &error);
    dump_gvdb_table(table, 0);
}

/**
 * The data stored in this file is equivalent to the following dict:
 * {
 *     "string": "test string",
 *     "table": {
 *         "int": uint32 42,
 *     }
 * }
 *
 * Test file 2 is big endian
 */
void create_test_file_2() {
    printf("Creating test file 2\n");
    GHashTable *root = gvdb_hash_table_new(NULL, NULL);

    GvdbItem *item = gvdb_hash_table_insert(root, "string");
    GVariant *string_value = g_variant_new_string("test string");
    gvdb_item_set_value(item, string_value);

    GHashTable *sub_table = gvdb_hash_table_new(root, "table");
    GVariant *int_value = g_variant_new_uint32(42);
    GvdbItem *int_item = gvdb_hash_table_insert(sub_table, "int");
    gvdb_item_set_value(int_item, int_value);

    GvdbItem *sub_table_item = gvdb_hash_table_insert(root, "table");
    gvdb_item_set_hash_table(sub_table_item, sub_table);

    GError *error = NULL;

    gvdb_table_write_contents(root, TEST_FILE_2, G_BYTE_ORDER == G_LITTLE_ENDIAN, &error);
}

void read_test_file_2() {
    GError *error = NULL;
    GvdbTable *table = gvdb_table_new(TEST_FILE_2, FALSE, &error);
    dump_gvdb_table(table, 0);
}

int main(int argc, char *argv[]) {
    create_test_file_1();
    read_test_file_1();
    create_test_file_2();
    read_test_file_2();
}
