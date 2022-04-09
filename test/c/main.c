#include <stdio.h>
#include <glib-2.0/glib.h>
#include "gvdb-builder.h"
#include "gvdb-reader.h"

#define TEST_PATH "../data/"
#define TEST_FILE_1 TEST_PATH "test1.gvdb"

void create_test_file() {
    printf("Creating test binary file\n");
    GHashTable *table = gvdb_hash_table_new(NULL, NULL);

    GvdbItem *item = gvdb_hash_table_insert(table, "root_key");
    GVariantBuilder builder;
    g_variant_builder_init(&builder, G_VARIANT_TYPE("(uus)"));
    g_variant_builder_add(&builder, "u", 1234);
    g_variant_builder_add(&builder, "u", 98765);
    GVariant *v_data = g_variant_new_string("TEST_STRING_VALUE");
    g_variant_builder_add_value(&builder, v_data);

    gvdb_item_set_value(item, g_variant_builder_end(&builder));

    GError *error = NULL;

    gvdb_table_write_contents(table, TEST_FILE_1, G_BYTE_ORDER != G_LITTLE_ENDIAN, &error);
}

void read_test_file() {
    GError *error = NULL;
    GvdbTable *table = gvdb_table_new(TEST_FILE_1, FALSE, &error);
    gsize length;
    gchar **names = gvdb_table_get_names(table, &length);
    int i = 0;
    printf("Reading file " TEST_FILE_1 "\n");
    while (names[i] != NULL) {
        printf("%s:\n", names[i]);
        GVariant *variant = gvdb_table_get_value(table, names[i]);
        if (variant != NULL) {
            printf("  GVariant: %s\n", g_variant_get_type_string(variant));
            printf("  Value: %s\n", g_variant_print(variant, TRUE));
        }
        i++;
    }
}

int main(int argc, char *argv[]) {
    create_test_file();
    read_test_file();
}
